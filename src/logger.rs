
pub use log::{self, Level as LogLevel, LevelFilter as LogLevelFilter};


pub struct LogFilter {
    pub default_level: LogLevel,

    #[cfg(not(target_family="wasm"))]
    pub module_filter: Option<&'static [(&'static str, LogLevelFilter)]>,

    #[cfg(target_family="wasm")]
    pub module_filter: Option<fn (&str) -> Option<LogLevelFilter>>,
}

impl From<LogLevel> for LogFilter {
    fn from(default_level: LogLevel) -> Self {
        Self { default_level, module_filter: None }
    }
}


#[macro_export]
macro_rules! log_filter {
    ($level:ident) => (LogLevel::$level);
    ($level:ident, $($module:literal => $module_level:ident),+) => {{

        #[cfg(not(target_family="wasm"))]
        let log_filter = $crate::log_filter!(@array: $level, $($module => $module_level,)+);

        #[cfg(target_family="wasm")]
        let log_filter = $crate::log_filter!(@fn: $level, $($module => $module_level,)+);

        log_filter
    }};
    (@array: $level:ident, $($module:literal => $module_level:ident,)+) => (LogFilter {
        default_level: $crate::LogLevel::$level,
        module_filter: Some(&[
            $( ($module, $crate::LogLevelFilter::$module_level) ,)+
        ]),
    });
    (@fn: $level:ident, $($module:literal => $module_level:ident,)+) => (LogFilter {
        default_level: $crate::LogLevel::$level,
        module_filter: Some(|module: &str| -> Option<$crate::LogLevelFilter> {
            match module {
                $( $module => Some($crate::LogLevelFilter::$module_level), )+
                _ => None,
            }
        }),
    });
}


#[cfg(target_family="wasm")]
pub(crate) struct LogFilterFn(pub(crate) fn (&str) -> Option<LogLevelFilter>);

#[cfg(target_family="wasm")]
impl log::Log for LogFilterFn {

    #[inline]
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    #[inline]
    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        if let Some(level) = self.0(record.target()) && record.level() > level {
            return;
        }

        console_log::log(record);
    }

    fn flush(&self) {}
}


use anyhow::{Result as Res};

impl LogFilter {

    pub fn init(self) -> Res<()> {

        let level_filter = self.default_level.to_level_filter();

        #[cfg(not(target_family="wasm"))] {

            let mut logger = simple_logger::SimpleLogger::new().with_level(level_filter);

            if let Some(filters) = self.module_filter {
                for (module, level) in filters {
                    logger = logger.with_module_level(module, *level);
                }
            }

            logger.init()?;
        }


        #[cfg(target_family="wasm")] {

            if let Some(filter_fn) = self.module_filter {
                log::set_max_level(level_filter);
                log::set_boxed_logger(Box::new(LogFilterFn(filter_fn)))?;
            }
            else {
                console_log::init_with_level(self.default_level)?;
            }
        }

        Ok(())
    }
}