
pub use log::{self, Level as LogLevel, LevelFilter as LogLevelFilter};


pub struct LogFilter {
    pub default_level: LogLevel,

    #[cfg(not(target_family="wasm"))]
    pub module_filter: Option<&'static [(&'static str, LogLevelFilter)]>,

    #[cfg(target_family="wasm")]
    pub module_filter: Option<fn (&str) -> Option<LogLevelFilter>>,

    pub tag: Option<&'static str>,
}

impl From<LogLevel> for LogFilter {
    fn from(default_level: LogLevel) -> Self {
        Self { default_level, module_filter: None, tag: None }
    }
}


#[macro_export]
macro_rules! log_filter {
    ($level:ident) => ($crate::LogLevel::$level);
    ($tag:expr, $level:ident) => {{
        let mut log_filter = $crate::LogFilter::from($crate::LogLevel::$level);
        log_filter.tag = Some($tag);
        log_filter
    }};
    ($level:ident, $($module:expr => $module_level:ident),+) => {{

        let mut log_filter = $crate::LogFilter::from($crate::LogLevel::$level);

        #[cfg(not(target_family="wasm"))] {
            log_filter.module_filter = Some($crate::log_filter!(@mf_array $($module => $module_level,)+));
        }

        #[cfg(target_family="wasm")] {
            log_filter.module_filter = Some($crate::log_filter!(@mf_fn $($module => $module_level,)+));
        }

        log_filter
    }};
    ($tag:expr, $level:ident, $($module:expr => $module_level:ident),+) => {{
        let mut log_filter = $crate::log_filter!($level, $($module => $module_level),+);
        log_filter.tag = Some($tag);
        log_filter
    }};
    (@mf_array $($module:expr => $module_level:ident,)+) => (
        &[
            $( ($module, $crate::LogLevelFilter::$module_level) ,)+
        ]
    );
    (@mf_fn $($module:expr => $module_level:ident,)+) => (
        |module: &str| -> Option<$crate::LogLevelFilter> {
            match module {
                $( $module => Some($crate::LogLevelFilter::$module_level), )+
                _ => None,
            }
        }
    );
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

        #[cfg(not(any(target_family="wasm", target_os="android")))] {

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

        #[cfg(target_os="android")] {

            let mut config = android_logger::Config::default().with_max_level(level_filter);

            if let Some(tag) = self.tag {
                config = config.with_tag(tag);
            }

            if let Some(filters) = self.module_filter {

                let mut builder = android_logger::FilterBuilder::new();

                for (module, level) in filters {
                    builder.filter_module(module, *level);
                }

                config = config.with_filter(builder.build());
            }

            android_logger::init_once(config);
        }

        Ok(())
    }
}