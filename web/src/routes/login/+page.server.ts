import { redirect } from "@sveltejs/kit";

export async function load({ fetch }) {
    // Fetch the redirect url from the backend /login endpoint
    const res = await fetch("http://localhost:3000/login", {
        redirect: "manual",
    });

    if (res.status === 303) {
        const location = res.headers.get("location") ?? res.url;
        throw redirect(303, location);
    }
}