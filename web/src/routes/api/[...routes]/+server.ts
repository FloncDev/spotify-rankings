import type { RequestHandler, RequestEvent } from "./$types"

const RUST_API = "http://localhost:3000";

async function proxy({ request, params, fetch, url }: RequestEvent) {
    console.log(`Proxying request to ${RUST_API}/${params.routes + url.search}`);
    const res = await fetch(`${RUST_API}/${params.routes + url.search}`, {
        method: request.method,
        headers: {
            ...Object.fromEntries(request.headers),
        },
        body: request.method === 'GET' || request.method === 'HEAD'
            ? undefined
            : await request.arrayBuffer(),
    });

    const headers = new Headers(res.headers);
    return new Response(res.body, {
        status: res.status,
        headers,
    });
}

export const GET: RequestHandler = proxy;
export const POST: RequestHandler = proxy;
export const PUT: RequestHandler = proxy;
export const PATCH: RequestHandler = proxy;
export const DELETE: RequestHandler = proxy;