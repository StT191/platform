
pub use log::{self, Level as LogLevel, LevelFilter as LogLevelFilter};


pub struct LogFilter {
    pub default_filter: LogLevelFilter,
    pub module_filter: Option<&'static [(&'static str, LogLevelFilter)]>,
    pub tag: Option<&'static str>,
}

impl From<LogLevelFilter> for LogFilter {
    fn from(default_filter: LogLevelFilter) -> Self { Self { default_filter, module_filter: None, tag: None } }
}

impl From<LogLevel> for LogFilter {
    fn from(default_level: LogLevel) -> Self { Self::from(default_level.to_level_filter()) }
}

#[macro_export]
macro_rules! log_filter {
    ($level:ident) => ($crate::LogLevelFilter::$level);
    ($tag:expr, $level:ident) => {
        $crate::LogFilter {
            default_filter: $crate::LogLevelFilter::$level,
            module_filter: None,
            tag: Some($tag),
        }
    };
    ($level:ident, $( $(@[$at:meta])? $module:expr => $module_level:ident ),+ ) => {{
        $crate::LogFilter {
            default_filter: $crate::LogLevelFilter::$level,
            module_filter: Some(&[
                $( $(#[$at])? ($module, $crate::LogLevelFilter::$module_level), )+
            ]),
            tag: None,
        }
    }};
    ($tag:expr, $level:ident, $( $(@[$at:meta])? $module:expr => $module_level:ident ),+ ) => {{
        let mut log_filter = $crate::log_filter!($level, $( $(@[$at])? $module => $module_level ),+ );
        log_filter.tag = Some($tag);
        log_filter
    }};
}


#[cfg(target_family="wasm")]
struct FilteredConsoleLogger {
    default_filter: LogLevelFilter,
    module_filter: &'static [(&'static str, LogLevelFilter)],
}

#[cfg(target_family="wasm")]
impl log::Log for FilteredConsoleLogger {

    #[inline]
    fn enabled(&self, metadata: &log::Metadata) -> bool {

        let filter = self.module_filter.iter().copied()
            .find(|(module, _)| metadata.target().starts_with(module))
            .map(|(_, level)| level)
            .unwrap_or(self.default_filter)
        ;

        metadata.level() <= filter
    }

    #[inline]
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            console_log::log(record);
        }
    }

    #[inline]
    fn flush(&self) {}
}


use anyhow::{Result as Res};

impl LogFilter {

    pub fn init(self) -> Res<()> {

        #[cfg(not(any(target_family="wasm", target_os="android")))] {

            let mut logger = simple_logger::SimpleLogger::new().with_level(self.default_filter);

            if let Some(filters) = self.module_filter {
                for (target, level) in filters {
                    logger = logger.with_module_level(target, *level);
                }
            }

            logger.init()?;
        }

        #[cfg(target_family="wasm")] {

            if let Some(module_filter) = self.module_filter {
                log::set_max_level(LogLevelFilter::Trace);
                log::set_boxed_logger(Box::new(FilteredConsoleLogger {
                    default_filter: self.default_filter,
                    module_filter,
                }))?;
            }
            else {
                log::set_max_level(self.default_filter);
                console_log::init()?;
            }
        }

        #[cfg(target_os="android")] {

            let mut config = android_logger::Config::default();

            if let Some(tag) = self.tag {
                config = config.with_tag(tag);
            }

            if let Some(filters) = self.module_filter {

                log::set_max_level(LogLevelFilter::Trace);

                let mut builder = android_logger::FilterBuilder::new();

                builder.filter_level(self.default_filter);

                for (target, level) in filters {
                    builder.filter_module(target, *level);
                }

                config = config.with_filter(builder.build());
            }
            else {
                config = config.with_max_level(self.default_filter);
            }

            android_logger::init_once(config);
        }

        Ok(())
    }
}