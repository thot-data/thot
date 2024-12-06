use super::{
    graph::{NodeMap, Tree},
    HasName, Ptr, Reducible,
};
use std::{
    ffi::{OsStr, OsString},
    path::{Component, Path, PathBuf},
};

pub const CORRUPTION_STR: &str = "[corrupt]";

pub type FileMap = NodeMap<File>;

#[derive(Debug, Clone)]
pub enum Action {
    CreateFolder { path: PathBuf, with_parents: bool },
    CreateFile { path: PathBuf, with_parents: bool },
    Remove(PathBuf),
    Rename { from: PathBuf, to: OsString },
    Move { from: PathBuf, to: PathBuf },
    Copy { from: PathBuf, to: PathBuf },
    Modify { file: PathBuf, kind: ModifyKind },
}

#[derive(Debug, Clone)]
pub enum ModifyKind {
    /// Add an item to a manifest.
    ManifestAdd(String),

    /// Remove an item from a manifest.
    ManifestRemove(usize),
    Corrupt,
    Repair,

    /// Initialize the file to its default content based on
    /// its app content.
    Initialize,

    Other,
}

#[derive(Debug)]
pub struct State {
    /// Path to root.
    path: PathBuf,
    graph: Tree<Folder>,
}

impl State {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let root = Folder::new(Component::RootDir.as_os_str());
        let graph = Tree::new(root);
        Self {
            path: path.into(),
            graph,
        }
    }

    pub fn base_path(&self) -> &PathBuf {
        &self.path
    }

    pub fn graph(&self) -> &Tree<Folder> {
        &self.graph
    }

    pub fn graph_mut(&mut self) -> &mut Tree<Folder> {
        &mut self.graph
    }
}

impl State {
    /// Returns the base path preprended to the given path.
    ///
    /// # Panics
    /// + If `path` is not absolute.
    /// + If `path` contains  `parent` directory (e.g. `..`).
    pub fn join_path(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        assert!(path.is_absolute());

        let rel_path = path
            .components()
            .into_iter()
            .filter(|component| match component {
                Component::Prefix(_) | Component::RootDir | Component::CurDir => false,
                Component::ParentDir => panic!("invalid path, contains parent directory"),
                Component::Normal(_) => true,
            })
            .fold(PathBuf::new(), |p, c| p.join(c));

        self.base_path().join(rel_path)
    }

    pub fn all_folders(&self) -> Vec<Ptr<Folder>> {
        self.graph.nodes().iter().cloned().collect()
    }

    pub fn all_files(&self) -> Vec<Ptr<File>> {
        self.graph
            .nodes()
            .iter()
            .flat_map(|folder| folder.borrow().files().iter().cloned().collect::<Vec<_>>())
            .collect()
    }

    pub fn find_folder(&self, path: impl AsRef<Path>) -> Option<Ptr<Folder>> {
        self.graph.find_by_path(path)
    }

    pub fn find_file(&self, path: impl AsRef<Path>) -> Option<Ptr<File>> {
        let path = path.as_ref();
        let filename = path.file_name()?;
        let parent = self.find_folder(path.parent()?)?;
        let file = parent
            .borrow()
            .files
            .iter()
            .find(|file| file.borrow().name == filename)?
            .clone();

        Some(file)
    }

    /// Returns whether the path exists.
    pub fn exists(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        let Some(name) = path.file_name() else {
            return false;
        };
        let Some(parent) = path.parent() else {
            return false;
        };

        let Some(parent) = self.find_folder(parent) else {
            return false;
        };

        self.name_exists(&parent, name.to_os_string())
            .unwrap_or(false)
    }

    /// # Returns
    /// Whether the parent folder contains a resource with the given name.
    /// `None` if the parent does not exist.
    pub fn name_exists(&self, parent: &Ptr<Folder>, name: impl AsRef<OsStr>) -> Option<bool> {
        if !self.graph.contains(parent) {
            return None;
        }

        let name = name.as_ref();
        if parent.borrow().file(name).is_some() {
            return Some(true);
        }

        let Some(children) = self.graph.children(&parent) else {
            return Some(false);
        };

        let name: &OsStr = name.as_ref();
        Some(
            children
                .iter()
                .find(|child| child.borrow().name == name)
                .is_some(),
        )
    }

    /// Find a folder that contains the given file reference.
    pub fn find_file_folder_by_ptr(&self, file: &Ptr<File>) -> Option<&Ptr<Folder>> {
        self.graph().nodes().iter().find(|folder| {
            folder
                .borrow()
                .files()
                .iter()
                .find(|f| Ptr::ptr_eq(f, file))
                .is_some()
        })
    }

    /// # Returns
    /// The path to the file, relative to the graph root.
    pub fn file_path(&self, file: &Ptr<File>) -> Option<PathBuf> {
        let folder = self.find_file_folder_by_ptr(file)?;
        let mut path = self.graph.path(folder)?;
        path.push(file.borrow().name());
        Some(path)
    }
}

impl State {
    /// Duplicate the state.
    pub fn duplicate_with_app_references_and_map(&self) -> (Self, NodeMap<Folder>) {
        let (graph, node_map) = self.graph.duplicate_with_map();
        (
            Self {
                path: self.path.clone(),
                graph,
            },
            node_map,
        )
    }
}

impl Reducible for State {
    type Action = Action;
    fn reduce(&mut self, action: Self::Action) {
        match action {
            Action::CreateFolder { path, with_parents } => {
                assert!(!self.exists(&path));
                let parent = if with_parents {
                    let mut current_path = PathBuf::new();
                    let mut current_parent = self.graph().root();
                    for component in path.parent().unwrap().components() {
                        match component {
                            Component::RootDir | Component::CurDir => {}
                            Component::Normal(segment) => {
                                current_path.push(segment);
                                current_parent =
                                    if let Some(parent) = self.find_folder(&current_path) {
                                        parent
                                    } else {
                                        self.graph_mut()
                                            .insert(Folder::new(segment), &current_parent)
                                            .unwrap()
                                    };
                            }
                            _ => panic!("invalid path {:?}", path),
                        }
                    }

                    current_parent
                } else {
                    self.find_folder(path.parent().unwrap()).unwrap()
                };

                self.graph
                    .insert(Folder::new(path.file_name().unwrap()), &parent)
                    .unwrap();
            }

            Action::CreateFile { path, with_parents } => {
                assert!(!self.exists(&path));
                let parent = if with_parents {
                    let mut current_path = PathBuf::new();
                    let mut current_parent = self.graph().root();
                    for component in path.parent().unwrap().components() {
                        match component {
                            Component::RootDir | Component::CurDir => {}
                            Component::Normal(segment) => {
                                current_path.push(segment);
                                current_parent =
                                    if let Some(parent) = self.find_folder(&current_path) {
                                        parent
                                    } else {
                                        self.graph_mut()
                                            .insert(Folder::new(segment), &current_parent)
                                            .unwrap()
                                    };
                            }
                            _ => panic!("invalid path {:?}", path),
                        }
                    }

                    current_parent
                } else {
                    self.find_folder(path.parent().unwrap()).unwrap()
                };

                parent
                    .borrow_mut()
                    .insert(File::new(path.file_name().unwrap()))
                    .unwrap();
            }

            Action::Remove(path) => {
                if let Some(file) = self.find_file(&path) {
                    let parent = self.find_file_folder_by_ptr(&file).unwrap();
                    assert!(parent.borrow_mut().remove(&file));
                } else if let Some(folder) = self.find_folder(&path) {
                    assert!(self.graph.remove(&folder).is_some());
                } else {
                    panic!("invalid path resource");
                }
            }

            Action::Rename { from, to } => {
                if let Some(file) = self.find_file(&from) {
                    let parent = self.find_file_folder_by_ptr(&file).unwrap();
                    if self.name_exists(parent, &to).unwrap() {
                        panic!("name exists");
                    }

                    file.borrow_mut().set_name(to);
                } else if let Some(folder) = self.find_folder(from) {
                    let parent = self.graph.parent(&folder).unwrap();
                    if self.name_exists(&parent, &to).unwrap() {
                        panic!("name exists");
                    }

                    folder.borrow_mut().set_name(to);
                } else {
                    panic!("invalid path resource");
                }
            }

            Action::Move { from, to } => {
                let parent_new = self.find_folder(to.parent().unwrap()).unwrap();
                if let Some(file) = self.find_file(&from) {
                    let parent_old = self.find_file_folder_by_ptr(&file).unwrap();
                    let filename = to.file_name().unwrap();
                    if self.name_exists(&parent_new, filename).unwrap() {
                        panic!("invalid path resource");
                    }

                    file.borrow_mut().set_name(filename);
                    parent_old.borrow_mut().remove(&file);
                    parent_new.borrow_mut().insert_ptr(file.clone()).unwrap();
                } else if let Some(folder) = self.find_folder(&from) {
                    let _parent_old = self.graph.parent(&folder).unwrap();
                    if self
                        .name_exists(&parent_new, to.file_name().unwrap())
                        .unwrap()
                    {
                        panic!("invalid path resource");
                    }

                    let graph = self.graph.remove(&folder).unwrap();
                    self.graph.insert_tree(graph, &parent_new).unwrap();
                } else {
                    panic!("invalid path resource");
                }
            }

            Action::Copy { from, to } => {
                let parent_new = self.find_folder(to.parent().unwrap()).unwrap();
                if let Some(file) = self.find_file(&from) {
                    let _parent_old = self.find_file_folder_by_ptr(&file).unwrap();
                    if self
                        .name_exists(&parent_new, to.file_name().unwrap())
                        .unwrap()
                    {
                        panic!("invalid path resource");
                    }

                    let mut file = file.borrow().clone();
                    file.set_name(to.file_name().unwrap());
                } else if let Some(folder) = self.find_folder(&from) {
                    let _parent_old = self.graph.parent(&folder).unwrap();
                    if self
                        .name_exists(&parent_new, to.file_name().unwrap())
                        .unwrap()
                    {
                        panic!("invalid path resource");
                    }

                    let dup = self.graph.duplicate_subtree(&folder).unwrap();
                    self.graph.insert_tree(dup, &parent_new).unwrap();
                } else {
                    panic!("invalid path resource");
                }
            }

            Action::Modify { file, kind } => match kind {
                ModifyKind::Corrupt => {
                    let file = self.find_file(file).unwrap();
                    let mut file = file.borrow_mut();
                    assert!(!file.content().ends_with(CORRUPTION_STR));
                    file.append(CORRUPTION_STR);
                }
                ModifyKind::Repair => {
                    let file = self.find_file(file).unwrap();
                    let mut file = file.borrow_mut();
                    assert!(file.content().ends_with(CORRUPTION_STR));
                    let mut content = file.content().clone();
                    content.truncate(content.len() - CORRUPTION_STR.len());
                    file.write(content);
                }

                // left for the parent state to handle
                ModifyKind::ManifestAdd(_) => {}
                ModifyKind::ManifestRemove(_) => {}
                ModifyKind::Initialize | ModifyKind::Other => {}
            },
        }
    }
}

#[derive(Debug, Clone, derive_more::From)]
pub enum Resource {
    File(Ptr<File>),
    Folder(Ptr<Folder>),
}

#[derive(Debug)]
pub struct Folder {
    name: OsString,
    files: Vec<Ptr<File>>,
}

impl Folder {
    pub fn new(name: impl Into<OsString>) -> Self {
        Self {
            name: name.into(),
            files: vec![],
        }
    }

    pub fn files(&self) -> &Vec<Ptr<File>> {
        &self.files
    }

    /// Gets the file with the given name.
    pub fn file(&self, name: impl AsRef<OsStr>) -> Option<Ptr<File>> {
        let name = name.as_ref();
        let file = self
            .files
            .iter()
            .find(|file| file.borrow().name == name)?
            .clone();

        Some(file)
    }

    /// Insert a file.
    pub fn insert(&mut self, file: File) -> super::Result<Ptr<File>> {
        if self.file(file.name()).is_some() {
            return Err(super::error::Error::NameCollision);
        }

        let file = Ptr::new(file);
        self.files.push(file.clone());
        Ok(file)
    }

    /// Inserts a file.
    pub fn insert_ptr(&mut self, file: Ptr<File>) -> super::Result {
        if self.file(file.borrow().name()).is_some() {
            return Err(super::error::Error::NameCollision);
        }

        self.files.push(file.clone());
        Ok(())
    }

    /// # Returns
    /// `true` if the file existed in the folder and was removed,
    /// `false` if the file did not exist in the folder.
    pub fn remove(&mut self, file: &Ptr<File>) -> bool {
        let Some(index) = self.files.iter().position(|f| Ptr::ptr_eq(f, file)) else {
            return false;
        };

        self.files.swap_remove(index);
        return true;
    }
}

impl Folder {
    /// Duplicates the folder.
    /// New `File`s are created.
    pub fn duplicate_with_app_resources_and_map(&self) -> (Self, FileMap) {
        let mut file_map = Vec::with_capacity(self.files.len());
        let files = self
            .files
            .iter()
            .map(|file| {
                let dup = Ptr::new(file.borrow().clone());
                file_map.push((file.clone(), dup.clone()));
                dup
            })
            .collect();

        (
            Self {
                name: self.name.clone(),
                files,
            },
            file_map,
        )
    }
}

impl Clone for Folder {
    fn clone(&self) -> Self {
        let (dup, _file_map) = self.duplicate_with_app_resources_and_map();
        dup
    }
}

#[derive(Debug, Clone)]
pub struct File {
    name: OsString,
    content: String,
}

impl File {
    pub fn new(name: impl Into<OsString>) -> Self {
        Self {
            name: name.into(),
            content: String::new(),
        }
    }

    pub fn content(&self) -> &String {
        &self.content
    }

    /// Replaces the file's content.
    pub fn write(&mut self, content: impl Into<String>) {
        self.content = content.into();
    }

    pub fn append(&mut self, content: impl AsRef<str>) {
        self.content.push_str(content.as_ref())
    }
}

impl HasName for Folder {
    fn name(&self) -> &std::ffi::OsStr {
        &self.name
    }

    fn set_name(&mut self, name: impl Into<OsString>) {
        self.name = name.into()
    }
}

impl HasName for File {
    fn name(&self) -> &std::ffi::OsStr {
        &self.name
    }

    fn set_name(&mut self, name: impl Into<OsString>) {
        self.name = name.into()
    }
}
