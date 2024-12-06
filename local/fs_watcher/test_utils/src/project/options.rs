use std::marker::PhantomData;
use syre_core::{
    graph::ResourceTree,
    project::{Container, Project as CoreProject},
    types::Creator,
};

#[cfg(feature = "fs")]
use syre_local::project::resources::{Container as LocalContainer, Project as LocalProject};

#[cfg(feature = "fs")]
pub use fs::*;

pub type ContainerTree = ResourceTree<Container>;

pub trait OptionsSpecs {
    fn with_fs(&self) -> bool;
    fn with_assets(&self) -> bool;
    fn with_assets_fs(&self) -> bool;
    fn with_analysis(&self) -> bool;
    fn with_analysis_fs(&self) -> bool;
}

pub trait Build {
    fn build<D, A>(options: &Options<NoFs, D, A>) -> Project<CoreProject, ContainerTree>
    where
        Options<NoFs, D, A>: OptionsSpecs;

    #[cfg(feature = "fs")]
    fn build_fs<D, A>(
        options: &Options<Fs, D, A>,
        base_path: impl Into<std::path::PathBuf>,
    ) -> Result<Project<LocalProject, LocalContainerTree>, std::io::Error>
    where
        Options<Fs, D, A>: OptionsSpecs;
}

pub struct Project<P, G> {
    pub project: P,
    pub graph: G,
}

/// Marker.
/// Resource are only in memory.
#[derive(Default)]
pub struct NoFs;

/// Marker.
/// Include assets in the graph.
#[derive(Default)]
pub struct Assets<F> {
    marker_fs: PhantomData<F>,
}

/// Marker.
/// Do not include assets in the graph.
#[derive(Default)]
pub struct NoAssets;

/// Marker.
/// Include analyses in the project.
#[derive(Default)]
pub struct Analysis<F> {
    marker_fs: PhantomData<F>,
}

/// Marker.
/// Do not include analyses in the project.
#[derive(Default)]
pub struct NoAnalysis;

/// Basic fireworks project.
#[derive(Default)]
pub struct Options<F, D, A> {
    creator: Option<Creator>,
    marker_fs: PhantomData<F>,
    marker_assets: PhantomData<D>,
    marker_analysis: PhantomData<A>,
}

impl<F, D, A> Options<F, D, A> {
    pub fn creator(&self) -> &Option<Creator> {
        &self.creator
    }

    pub fn set_creator(&mut self, creator: Creator) {
        let _ = self.creator.insert(creator);
    }
}

impl<D, A> Options<NoFs, D, A> {
    pub fn build<G: Build>(&self) -> Project<CoreProject, ContainerTree>
    where
        Options<NoFs, D, A>: OptionsSpecs,
    {
        G::build(&self)
    }
}

impl Options<NoFs, NoAssets, NoAnalysis> {
    pub fn new() -> Options<NoFs, NoAssets, NoAnalysis> {
        Self::default()
    }
}

impl<F, A> Options<F, NoAssets, A> {
    /// Include assets in the graph.
    pub fn with_assets(self) -> Options<F, Assets<NoFs>, A> {
        Options {
            creator: self.creator,
            marker_fs: PhantomData,
            marker_assets: PhantomData,
            marker_analysis: self.marker_analysis,
        }
    }
}

impl<F, AF, A> Options<F, Assets<AF>, A> {
    /// Do not include assets in the graph.
    pub fn without_assets(self) -> Options<F, NoAssets, A> {
        Options {
            creator: self.creator,
            marker_fs: PhantomData,
            marker_assets: PhantomData,
            marker_analysis: PhantomData::<A>,
        }
    }
}

impl<F, D> Options<F, D, NoAnalysis> {
    /// Include and assign analyses to the graph.
    pub fn with_analysis(self) -> Options<F, D, Analysis<NoFs>> {
        Options {
            creator: self.creator,
            marker_fs: PhantomData,
            marker_assets: PhantomData::<D>,
            marker_analysis: PhantomData,
        }
    }
}

impl<F, D, AF> Options<F, D, Analysis<AF>> {
    /// Do not include analyses in the project.
    pub fn without_analysis(self) -> Options<F, D, NoAnalysis> {
        Options {
            creator: self.creator,
            marker_fs: PhantomData,
            marker_assets: PhantomData::<D>,
            marker_analysis: PhantomData,
        }
    }
}

impl OptionsSpecs for Options<NoFs, NoAssets, NoAnalysis> {
    fn with_fs(&self) -> bool {
        false
    }

    fn with_assets(&self) -> bool {
        false
    }

    fn with_assets_fs(&self) -> bool {
        false
    }

    fn with_analysis(&self) -> bool {
        false
    }

    fn with_analysis_fs(&self) -> bool {
        false
    }
}

impl OptionsSpecs for Options<NoFs, Assets<NoFs>, NoAnalysis> {
    fn with_fs(&self) -> bool {
        false
    }

    fn with_assets(&self) -> bool {
        true
    }

    fn with_assets_fs(&self) -> bool {
        false
    }

    fn with_analysis(&self) -> bool {
        false
    }

    fn with_analysis_fs(&self) -> bool {
        false
    }
}

impl OptionsSpecs for Options<NoFs, NoAssets, Analysis<NoFs>> {
    fn with_fs(&self) -> bool {
        false
    }

    fn with_assets(&self) -> bool {
        false
    }

    fn with_assets_fs(&self) -> bool {
        false
    }

    fn with_analysis(&self) -> bool {
        true
    }

    fn with_analysis_fs(&self) -> bool {
        false
    }
}

impl OptionsSpecs for Options<NoFs, Assets<NoFs>, Analysis<NoFs>> {
    fn with_fs(&self) -> bool {
        false
    }

    fn with_assets(&self) -> bool {
        true
    }

    fn with_assets_fs(&self) -> bool {
        false
    }

    fn with_analysis(&self) -> bool {
        true
    }

    fn with_analysis_fs(&self) -> bool {
        false
    }
}

#[cfg(feature = "fs")]
mod fs {
    use super::*;
    use std::path::PathBuf;

    pub type LocalContainerTree = ResourceTree<LocalContainer>;

    /// Marker.
    /// Write Resources to the file system.
    #[derive(Default)]
    pub struct Fs;

    impl<D, A> Options<Fs, D, A> {
        pub fn build<G: Build>(
            &self,
            base_path: impl Into<PathBuf>,
        ) -> Result<Project<LocalProject, LocalContainerTree>, std::io::Error>
        where
            Options<Fs, D, A>: OptionsSpecs,
        {
            G::build_fs(&self, base_path)
        }
    }

    impl<D, A> Options<NoFs, D, A> {
        /// Write the graph to the file system.
        pub fn with_fs(self) -> Options<Fs, D, A> {
            Options {
                creator: self.creator,
                marker_fs: PhantomData,
                marker_assets: PhantomData,
                marker_analysis: PhantomData,
            }
        }
    }

    impl Options<Fs, NoAssets, NoAnalysis> {
        /// Do not write the graph to the file system.
        /// Nothing else may be written to the file system, either.
        pub fn without_fs(self) -> Options<NoFs, NoAssets, NoAnalysis> {
            Options {
                creator: self.creator,
                marker_fs: PhantomData,
                marker_assets: PhantomData,
                marker_analysis: PhantomData,
            }
        }
    }

    impl<F> Options<Fs, Assets<F>, NoAnalysis> {
        /// Do not write the graph to the file system.
        /// Nothing else may be written to the file system, either.
        pub fn without_fs(self) -> Options<NoFs, Assets<NoFs>, NoAnalysis> {
            Options {
                creator: self.creator,
                marker_fs: PhantomData,
                marker_assets: PhantomData,
                marker_analysis: PhantomData,
            }
        }
    }

    impl<F> Options<Fs, NoAssets, Analysis<F>> {
        /// Do not write the graph to the file system.
        /// Nothing else may be written to the file system, either.
        pub fn without_fs(self) -> Options<NoFs, NoAssets, Analysis<NoFs>> {
            Options {
                creator: self.creator,
                marker_fs: PhantomData,
                marker_assets: PhantomData,
                marker_analysis: PhantomData,
            }
        }
    }

    impl<DF, AF> Options<Fs, Assets<DF>, Analysis<AF>> {
        /// Do not write the graph to the file system.
        /// Nothing else may be written to the file system, either.
        pub fn without_fs(self) -> Options<NoFs, Assets<NoFs>, Analysis<NoFs>> {
            Options {
                creator: self.creator,
                marker_fs: PhantomData,
                marker_assets: PhantomData,
                marker_analysis: PhantomData,
            }
        }
    }

    impl<A> Options<Fs, Assets<NoFs>, A> {
        /// Write asset files to the file system.
        pub fn with_asset_files(self) -> Options<Fs, Assets<Fs>, A> {
            Options {
                creator: self.creator,
                marker_fs: PhantomData,
                marker_assets: PhantomData,
                marker_analysis: PhantomData::<A>,
            }
        }
    }

    impl<A> Options<Fs, Assets<Fs>, A> {
        /// Do not write asset files to the file system.
        pub fn without_asset_files(self) -> Options<Fs, Assets<NoFs>, A> {
            Options {
                creator: self.creator,
                marker_fs: PhantomData,
                marker_assets: PhantomData,
                marker_analysis: PhantomData::<A>,
            }
        }
    }

    impl<D> Options<Fs, D, Analysis<NoFs>> {
        /// Write analysis files to the file system.
        pub fn with_analysis_files(self) -> Options<Fs, D, Analysis<Fs>> {
            Options {
                creator: self.creator,
                marker_fs: PhantomData,
                marker_assets: PhantomData::<D>,
                marker_analysis: PhantomData,
            }
        }
    }

    impl<D> Options<Fs, D, Analysis<NoFs>> {
        /// Do not write analysis file to the file system.
        pub fn without_analysis_files(self) -> Options<Fs, D, Analysis<NoFs>> {
            Options {
                creator: self.creator,
                marker_fs: PhantomData,
                marker_assets: PhantomData::<D>,
                marker_analysis: PhantomData,
            }
        }
    }

    impl OptionsSpecs for Options<Fs, Assets<NoFs>, NoAnalysis> {
        fn with_fs(&self) -> bool {
            true
        }

        fn with_assets(&self) -> bool {
            true
        }

        fn with_assets_fs(&self) -> bool {
            false
        }

        fn with_analysis(&self) -> bool {
            false
        }

        fn with_analysis_fs(&self) -> bool {
            false
        }
    }

    impl OptionsSpecs for Options<Fs, Assets<Fs>, NoAnalysis> {
        fn with_fs(&self) -> bool {
            true
        }

        fn with_assets(&self) -> bool {
            true
        }

        fn with_assets_fs(&self) -> bool {
            true
        }

        fn with_analysis(&self) -> bool {
            false
        }

        fn with_analysis_fs(&self) -> bool {
            false
        }
    }

    impl OptionsSpecs for Options<Fs, NoAssets, Analysis<NoFs>> {
        fn with_fs(&self) -> bool {
            true
        }

        fn with_assets(&self) -> bool {
            false
        }

        fn with_assets_fs(&self) -> bool {
            false
        }

        fn with_analysis(&self) -> bool {
            true
        }

        fn with_analysis_fs(&self) -> bool {
            false
        }
    }

    impl OptionsSpecs for Options<Fs, NoAssets, Analysis<Fs>> {
        fn with_fs(&self) -> bool {
            true
        }

        fn with_assets(&self) -> bool {
            false
        }

        fn with_assets_fs(&self) -> bool {
            false
        }

        fn with_analysis(&self) -> bool {
            true
        }

        fn with_analysis_fs(&self) -> bool {
            true
        }
    }

    impl OptionsSpecs for Options<Fs, Assets<NoFs>, Analysis<NoFs>> {
        fn with_fs(&self) -> bool {
            true
        }

        fn with_assets(&self) -> bool {
            true
        }

        fn with_assets_fs(&self) -> bool {
            false
        }

        fn with_analysis(&self) -> bool {
            true
        }

        fn with_analysis_fs(&self) -> bool {
            false
        }
    }

    impl OptionsSpecs for Options<Fs, Assets<Fs>, Analysis<Fs>> {
        fn with_fs(&self) -> bool {
            true
        }

        fn with_assets(&self) -> bool {
            true
        }

        fn with_assets_fs(&self) -> bool {
            true
        }

        fn with_analysis(&self) -> bool {
            true
        }

        fn with_analysis_fs(&self) -> bool {
            true
        }
    }
}
