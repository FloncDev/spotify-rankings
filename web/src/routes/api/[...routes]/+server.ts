import type { RequestHandler, RequestEvent } from "./$types"

const RUST_API = "http://localhost:3000";

async function proxy({ request, params, fetch, url }: RequestEvent) {
    const res = await fetch(`${RUST_API}/${params.routes + url.search}`, {
        method: request.method,
        headers: {
            ...Object.fromEntries(request.headers),
        },
        body: request.method === 'GET' || request.method === 'HEAD'
            ? undefined
            : await request.arrayBuffer(),
    });

    // Print the response status
    console.log(`Response status: ${res.status} for ${RUST_API}/${params.routes + url.search} `);

    // Pass response straight back to browser
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