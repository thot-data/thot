//! Simple fireworks project.
use super::options::{Build, ContainerTree, NoFs, Options, OptionsSpecs, Project};
use syre_core::{
    project::{
        asset::Builder as AssetBuilder, container::Builder as ContainerBuilder,
        AnalysisAssociation, Project as CoreProject, Script,
    },
    types::ResourceId,
};

#[cfg(feature = "fs")]
use super::options::{Fs, LocalContainerTree};

#[cfg(feature = "fs")]
use std::path::{Path, PathBuf};

#[cfg(feature = "fs")]
use syre_local::{
    graph::tree::ContainerTreeTransformer, project::resources::Project as LocalProject,
};

pub const SCRIPT_PATHS: [&'static str; 3] = [
    Fireworks::SCRIPT_RECIPE_COMPARISON_PATH,
    Fireworks::SCRIPT_RECIPE_STATS_PATH,
    Fireworks::SCRIPT_NOISE_STATS_PATH,
];

pub struct Fireworks;
impl Fireworks {
    pub const SCRIPT_RECIPE_COMPARISON_PATH: &'static str = "recipe_comparison.py";
    pub const SCRIPT_RECIPE_STATS_PATH: &'static str = "recipe_stats.py";
    pub const SCRIPT_NOISE_STATS_PATH: &'static str = "noise_stats.py";
}

impl Fireworks {
    fn build_project<F, D, A>(options: &Options<F, D, A>) -> Project<CoreProject, ContainerTree>
    where
        Options<F, D, A>: OptionsSpecs,
    {
        let mut project = CoreProject::new("fireworks");
        if options.with_analysis() {
            project.set_analysis_root("analysis");
        }

        let recipe_comparison = Script::from_path(Self::SCRIPT_RECIPE_COMPARISON_PATH).unwrap();
        let recipe_stats = Script::from_path(Self::SCRIPT_RECIPE_STATS_PATH).unwrap();
        let noise_stats = Script::from_path(Self::SCRIPT_NOISE_STATS_PATH).unwrap();
        let analyses = Analyses {
            recipe_comparison: recipe_comparison.rid().clone(),
            recipe_stats: recipe_stats.rid().clone(),
            noise_stats: noise_stats.rid().clone(),
        };

        let graph = Self::build_graph(options, analyses);
        project.data_root = graph
            .get(graph.root())
            .unwrap()
            .properties
            .name
            .clone()
            .into();

        Project { project, graph }
    }

    /// Create the graph.
    fn build_graph<F, D, A>(options: &Options<F, D, A>, analyses: Analyses) -> ContainerTree
    where
        Options<F, D, A>: OptionsSpecs,
    {
        let mut root = ContainerBuilder::new("data");
        if options.with_analysis() {
            root.insert_analysis(AnalysisAssociation::new(analyses.recipe_comparison.clone()));
        }
        let root = root.build();

        let mut recipe_a = ContainerBuilder::new("Recipe A");
        recipe_a.set_kind("recipe");
        recipe_a.set_metadatum("name", "A");
        if options.with_analysis() {
            recipe_a.insert_analysis(AnalysisAssociation::new(analyses.recipe_stats.clone()));
        }
        let recipe_a = recipe_a.build();

        let mut recipe_b = ContainerBuilder::new("Recipe B");
        recipe_b.set_kind("recipe");
        recipe_b.set_metadatum("name", "B");
        if options.with_analysis() {
            recipe_b.insert_analysis(AnalysisAssociation::new(analyses.recipe_stats.clone()));
        }
        let recipe_b = recipe_b.build();

        let mut batch_a1 = ContainerBuilder::new("Batch 1");
        batch_a1.set_kind("batch");
        batch_a1.set_metadatum("number", 1);
        if options.with_assets() {
            let mut data_a1 = AssetBuilder::with_path("a1_data.csv");
            data_a1.set_kind("noise-data");
            batch_a1.insert_asset(data_a1.build());
        }
        if options.with_analysis() {
            batch_a1.insert_analysis(AnalysisAssociation::new(analyses.noise_stats.clone()));
        }

        let batch_a1 = batch_a1.build();

        let mut batch_a2 = ContainerBuilder::new("Batch 2");
        batch_a2.set_kind("batch");
        batch_a2.set_metadatum("number", 2);
        if options.with_assets() {
            let mut data_a2 = AssetBuilder::with_path("a2_data.csv");
            data_a2.set_kind("noise-data");
            batch_a2.insert_asset(data_a2.build());
        }
        if options.with_analysis() {
            batch_a2.insert_analysis(AnalysisAssociation::new(analyses.noise_stats.clone()));
        }

        let batch_a2 = batch_a2.build();

        let mut batch_b1 = ContainerBuilder::new("Batch 1");
        batch_b1.set_kind("batch");
        batch_b1.set_metadatum("number", 1);
        if options.with_assets() {
            let mut data_b1 = AssetBuilder::with_path("a2_data.csv");
            data_b1.set_kind("noise-data");
            batch_b1.insert_asset(data_b1.build());
        }
        if options.with_analysis() {
            batch_b1.insert_analysis(AnalysisAssociation::new(analyses.noise_stats.clone()));
        }

        let batch_b1 = batch_b1.build();

        let mut batch_b2 = ContainerBuilder::new("Batch 2");
        batch_b2.set_kind("batch");
        batch_b2.set_metadatum("number", 2);
        if options.with_assets() {
            let mut data_b2 = AssetBuilder::with_path("a2_data.csv");
            data_b2.set_kind("noise-data");
            batch_b2.insert_asset(data_b2.build());
        }
        if options.with_analysis() {
            batch_b2.insert_analysis(AnalysisAssociation::new(analyses.noise_stats.clone()));
        }

        let batch_b2 = batch_b2.build();

        let root_id = root.rid().clone();
        let recipe_a_id = recipe_a.rid().clone();
        let recipe_b_id = recipe_b.rid().clone();

        let mut graph = ContainerTree::new(root);
        graph.insert(root_id.clone(), recipe_a).unwrap();
        graph.insert(root_id.clone(), recipe_b).unwrap();
        graph.insert(recipe_a_id.clone(), batch_a1).unwrap();
        graph.insert(recipe_a_id.clone(), batch_a2).unwrap();
        graph.insert(recipe_b_id.clone(), batch_b1).unwrap();
        graph.insert(recipe_b_id.clone(), batch_b2).unwrap();

        graph
    }
}

impl Build for Fireworks {
    fn build<D, A>(options: &Options<NoFs, D, A>) -> Project<CoreProject, ContainerTree>
    where
        Options<NoFs, D, A>: OptionsSpecs,
    {
        Self::build_project(options)
    }

    #[cfg(feature = "fs")]
    fn build_fs<D, A>(
        options: &Options<Fs, D, A>,
        base_path: impl Into<PathBuf>,
    ) -> Result<Project<LocalProject, LocalContainerTree>, std::io::Error>
    where
        Options<Fs, D, A>: OptionsSpecs,
    {
        let Project { project, graph } = Self::build_project(options);
        let base_path = base_path.into();
        let project = LocalProject::from(base_path.clone(), project);
        project.save().unwrap();

        let graph = ContainerTreeTransformer::core_to_local(graph, base_path);
        for node in graph.nodes().values() {
            node.save().unwrap();
            if options.with_assets_fs() {
                for asset in node.assets.iter() {
                    touch(&node.base_path().join(asset.path.clone()))?;
                }
            }
        }

        if options.with_analysis_fs() {
            std::fs::create_dir(project.analysis_root_path().unwrap())?;
            for path in SCRIPT_PATHS {
                touch(&project.analysis_root_path().unwrap().join(path))?;
            }
        }

        Ok(Project { project, graph })
    }
}

struct Analyses {
    pub recipe_comparison: ResourceId,
    pub recipe_stats: ResourceId,
    pub noise_stats: ResourceId,
}

#[cfg(feature = "fs")]
fn touch(path: &Path) -> std::io::Result<()> {
    match std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)
    {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
