export function getWebGlVendor(context: WebGL2RenderingContext): string {
    const ext = context.getExtension("WEBGL_debug_renderer_info")
    const fallback = context.getParameter(context.VENDOR)

    if (ext === null) {
        return fallback;
    }

    return context.getParameter(ext.UNMASKED_VENDOR_WEBGL) || fallback
}

export function getWebGlRenderer(context: WebGL2RenderingContext): string {
    const ext = context.getExtension("WEBGL_debug_renderer_info")
    const fallback = context.getParameter(context.RENDERER)

    if (ext === null) {
        return fallback;
    }

    return context.getParameter(ext.UNMASKED_RENDERER_WEBGL) || fallback
}

export function loseWebGlContext(context: WebGL2RenderingContext) {
    const ext = context.getExtension("WEBGL_lose_context");

    if (ext !== null) {
        ext.loseContext();
    }
}

export function restoreWebGlContext(context: WebGL2RenderingContext) {
    const ext = context.getExtension("WEBGL_lose_context");

    if (ext !== null) {
        ext.restoreContext();
    }
}

export class WebGlTimeElapsedQuery {
    private readonly pending: WebGLQuery[] = [];
    private readonly running: WebGLQuery[] = [];
    private readonly extension!: any | null;

    public constructor(private readonly context: WebGL2RenderingContext) {
        this.extension = context.getExtension("EXT_disjoint_timer_query_webgl2");
    }

    public clear() {
        for (const query in this.pending) {
            this.context.deleteQuery(query);
        }

        for (const query in this.running) {
            this.context.deleteQuery(query);
        }

        this.pending.length = 0;
        this.running.length = 0;
    }

    public timeElapsed(operation: () => void): number | null {
        if (this.extension === null || this.context.isContextLost()) {
            operation();
            return null;
        }

        const pendingQuery = this.pending.shift() || this.context.createQuery();

        if (pendingQuery !== null) {
            this.context.beginQuery(this.extension.TIME_ELAPSED_EXT, pendingQuery);
            operation();
            this.context.endQuery(this.extension.TIME_ELAPSED_EXT);
            this.running.push(pendingQuery);
        } else {
            operation();
            return null;
        }

        if (this.running.length < 2) {
            return null;
        }

        const runningQuery = this.running[0];

        const available = this.context.getQueryParameter(runningQuery, this.context.QUERY_RESULT_AVAILABLE) || false;

        if (available && !this.context.getParameter(this.extension.GPU_DISJOINT_EXT)) {
            const elapsed = this.context.getQueryParameter(runningQuery, this.context.QUERY_RESULT) as number;

            this.pending.push(this.running.shift()!);

            return elapsed / 1000000000;
        } else {
            return null;
        }
    }
}
