#[allow(unused_imports)]
use log::{debug, info, warn};

use web_sys::{ExtDisjointTimerQuery, WebGl2RenderingContext as Context, WebGlQuery};

#[derive(Debug)]
pub struct Query {
    gl: Context,
    handle: Option<WebGlQuery>,
    busy: bool,
}

impl Query {
    pub fn new(gl: Context) -> Self {
        Self {
            gl,
            handle: None,
            busy: false,
        }
    }

    pub(crate) fn reset(&mut self) {
        if Self::is_supported(&self.gl) {
            self.handle = self.gl.create_query();
            self.busy = false;
        } else {
            self.handle = None;
        }
    }

    pub fn start_query(&mut self) {
        if let Some(handle) = &self.handle {
            if !self.busy {
                self.gl
                    .begin_query(ExtDisjointTimerQuery::TIME_ELAPSED_EXT, handle);

                self.busy = true;
            }
        }
    }

    pub fn end_query(&mut self) -> Option<f32> {
        if self.busy {
            self.gl.end_query(ExtDisjointTimerQuery::TIME_ELAPSED_EXT);
        }

        if let Some(handle) = &self.handle {
            let available = self
                .gl
                .get_query_parameter(handle, Context::QUERY_RESULT_AVAILABLE)
                .as_bool()
                .unwrap_or_default();

            let disjoint = self
                .gl
                .get_parameter(ExtDisjointTimerQuery::GPU_DISJOINT_EXT)
                .unwrap()
                .as_bool()
                .unwrap_or_default();

            if available && !disjoint {
                let elapsed = self
                    .gl
                    .get_query_parameter(handle, Context::QUERY_RESULT)
                    .as_f64()
                    .unwrap();

                self.busy = false;

                return Some(elapsed as f32);
            }
        }

        None
    }

    pub fn query_time_elapsed(&mut self) -> QueryScope {
        QueryScope::begin(self)
    }

    pub fn is_supported(gl: &Context) -> bool {
        if let Ok(Some(_)) = gl.get_extension("EXT_disjoint_timer_query_webgl2") {
            true
        } else {
            false
        }
    }
}

impl Drop for Query {
    fn drop(&mut self) {
        self.gl.delete_query(self.handle.as_ref());
    }
}

pub struct QueryScope<'a> {
    query: &'a mut Query,
    is_running: bool,
}

impl<'a> QueryScope<'a> {
    fn begin(query: &'a mut Query) -> Self {
        let mut is_running = false;

        if let Some(handle) = &query.handle {
            if !query.busy {
                query
                    .gl
                    .begin_query(ExtDisjointTimerQuery::TIME_ELAPSED_EXT, handle);

                query.busy = true;
                is_running = true;
            }
        }

        Self { query, is_running }
    }

    pub fn end(self) -> Option<f32> {
        if self.is_running {
            self.query
                .gl
                .end_query(ExtDisjointTimerQuery::TIME_ELAPSED_EXT);
        }

        if let Some(handle) = &self.query.handle {
            let available = self
                .query
                .gl
                .get_query_parameter(handle, Context::QUERY_RESULT_AVAILABLE)
                .as_bool()
                .unwrap_or_default();

            let disjoint = self
                .query
                .gl
                .get_parameter(ExtDisjointTimerQuery::GPU_DISJOINT_EXT)
                .unwrap()
                .as_bool()
                .unwrap_or_default();

            if available && !disjoint {
                let elapsed = self
                    .query
                    .gl
                    .get_query_parameter(handle, Context::QUERY_RESULT)
                    .as_f64()
                    .unwrap();

                self.query.busy = false;

                return Some(elapsed as f32);
            }
        }

        None
    }
}
