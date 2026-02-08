<script lang="ts">
    import { isLoggedIn } from "$lib/stores";
    import { onMount } from "svelte";
    import { browser } from "$app/environment";
    import { goto } from "$app/navigation";

    let playlists: any = [];

    $: if (browser && !$isLoggedIn) {
        goto("/");
    }

    onMount(async () => {
        playlists = await fetch("/api/playlists")
            .then((response) => response.json())
            .catch((error) => {
                console.error("Failed to fetch playlists:", error);
                return [];
            });

        console.log("Fetched playlists:", playlists.length);
    });
</script>

<div class="flex flex-col items-center">
    <h1 class="text-5xl mb-4 text-white">Your Playlists</h1>
    <div class="flex flex-wrap justify-center">
        {#each playlists as playlist}
            <a
                href={`/playlists/${playlist.id}`}
                class="flex flex-col p-2 w-[300px] m-1 cursor-pointer bg-cyan-950 rounded hover:bg-cyan-800 transition no-underline"
                on:click|preventDefault={() =>
                    goto(`/playlists/${playlist.id}`)}
                tabindex="0"
            >
                <img
                    src={playlist.image_url}
                    alt={playlist.name}
                    class="w-100% rounded"
                />
                <h1 class="text-white text-xl">{playlist.name}</h1>
            </a>
        {/each}
    </div>
</div>
