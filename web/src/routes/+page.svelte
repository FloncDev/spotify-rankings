<!-- Create a centered button saying login with spotify and text above it saying welcome -->
<script>
    import { isLoggedIn } from "$lib/stores";
    import { onMount } from "svelte";
    import { browser } from "$app/environment";
    import { goto } from "$app/navigation";

    function login() {
        // Redirect the user directly to the backend login endpoint
        if (browser) {
            window.location.href = "/api/login";
        }
    }

    // React to changes in the isLoggedIn store
    $: if (browser && $isLoggedIn) {
        goto("/playlists");
    }

    onMount(() => {
        console.log("Initial logged in status:", $isLoggedIn);

        fetch("/api")
            .then((response) => response.text())
            .then((data) => {
                console.log("API response:", data);
            });
    });
</script>

<div class="flex flex-col items-center justify-center h-screen">
    <h1 class="text-5xl mb-4 text-white">Welcome :)</h1>
    <button
        class="px-6 py-2 bg-[#1f1f1f] rounded hover:bg-green-600 transition cursor-pointer"
        on:click={login}
    >
        <img
            src="/spotify_white.svg"
            alt="Spotify Logo"
            class="inline w-5 h-5 mr-2 align-middle"
        />
        Login with Spotify
    </button>
</div>
