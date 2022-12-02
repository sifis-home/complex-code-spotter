use arg_enum_proc_macro::ArgEnum;
use rust_code_analysis::FuncSpace;
use serde::Serialize;

trait ComplexityChecker {
    fn check(space: &FuncSpace, threshold: usize) -> Option<usize>;
}

struct Cyclomatic;

impl ComplexityChecker for Cyclomatic {
    fn check(space: &FuncSpace, threshold: usize) -> Option<usize> {
        let value = space.metrics.cyclomatic.cyclomatic() as usize;
        (value > threshold || space.metrics.cyclomatic.cyclomatic_max() as usize > threshold)
            .then_some(value)
    }
}

struct Cognitive;

impl ComplexityChecker for Cognitive {
    fn check(space: &FuncSpace, threshold: usize) -> Option<usize> {
        let value = space.metrics.cognitive.cognitive() as usize;
        (value > threshold || space.metrics.cognitive.cognitive_max() as usize > threshold)
            .then_some(value)
    }
}

/// Supported complexities metrics.
#[derive(ArgEnum, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Complexity {
    /// Cyclomatic metric.
    #[arg_enum(name = "cyclomatic")]
    Cyclomatic,
    /// Cognitive metric.
    #[arg_enum(name = "cognitive")]
    Cognitive,
}

impl Complexity {
    /// Default threshold for a metric.
    pub const fn default_threshold(&self) -> usize {
        match self {
            Self::Cyclomatic => 15,
            Self::Cognitive => 15,
        }
    }
    /// All complexity metrics.
    pub const fn all() -> &'static [Complexity] {
        &[Self::Cyclomatic, Self::Cognitive]
    }

    pub(crate) fn value(&self, space: &FuncSpace, threshold: usize) -> Option<usize> {
        match self {
            Self::Cyclomatic => Cyclomatic::check(space, threshold),
            Self::Cognitive => Cognitive::check(space, threshold),
        }
    }
}
