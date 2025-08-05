<script lang="ts">
    import { isLoggedIn } from "$lib/stores";
    import { onMount } from "svelte";
    import { browser } from "$app/environment";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";

    $: slug = page.params.slug;

    interface Song {
        href: string;
        id: string;
        name: string;
        artists: { name: string; href: string }[];
        image_url: string;
        rating: number;
        deviation: number;
        volatility: number;
        total_matches: number;
    }

    interface Match {
        song_a: Song;
        song_b: Song;
    }

    let leaderboard: Song[] = [];
    $: leaderboard;

    let match: Match | null = null;
    $: match;

    $: if (browser && !$isLoggedIn) {
        goto("/");
    } else {
        console.log("User is logged in");
        console.log("isLoggedIn:", $isLoggedIn);
    }

    onMount(async () => {
        // init tracks for playlist
        await fetch(`http://localhost:3000/playlists/${slug}`, {
            credentials: "include",
            method: "POST",
        }).catch(() => {
            console.error("Failed to fetch tracks");
        });

        leaderboard = await fetch(
            `http://localhost:3000/playlists/${slug}/leaderboard`,
            {
                credentials: "include",
                method: "GET",
            },
        )
            .then((response) => {
                if (!response.ok) {
                    throw new Error("Failed to fetch leaderboard");
                }
                return response.json();
            })

            .catch((error) => {
                console.error("Error fetching leaderboard:", error);
                return [];
            });

        match = await fetch(
            `http://localhost:3000/playlists/${slug}/matchmaking`,
            {
                credentials: "include",
                method: "GET",
            },
        )
            .then((response) => {
                if (!response.ok) {
                    throw new Error("Failed to fetch match");
                }
                return response.json();
            })
            .catch((error) => {
                console.error("Error fetching match:", error);
                return null;
            });
    });

    function choose_winner(songId: string) {
        fetch(`http://localhost:3000/playlists/${slug}/matchmaking`, {
            credentials: "include",
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                song_a: match?.song_a.id,
                song_b: match?.song_b.id,
                winner: songId,
            }),
        })
            .then((response) => {
                if (!response.ok) {
                    throw new Error("Failed to choose winner");
                }
            })
            .then(() => {
                // Fetch new leaderboard and match
                return Promise.all([
                    fetch(
                        `http://localhost:3000/playlists/${slug}/leaderboard`,
                        {
                            credentials: "include",
                            method: "GET",
                        },
                    ),
                    fetch(
                        `http://localhost:3000/playlists/${slug}/matchmaking`,
                        {
                            credentials: "include",
                            method: "GET",
                        },
                    ),
                ]);
            })
            .then(([leaderboardResponse, matchResponse]) => {
                if (!leaderboardResponse.ok || !matchResponse.ok) {
                    throw new Error(
                        "Failed to fetch updated leaderboard or match",
                    );
                }
                return Promise.all([
                    leaderboardResponse.json(),
                    matchResponse.json(),
                ]);
            })
            .then(([updatedLeaderboard, updatedMatch]) => {
                leaderboard = updatedLeaderboard;
                match = updatedMatch;
            })
            .catch((error) => {
                console.error("Error choosing winner:", error);
            });
    }
</script>

<table class="table-auto w-full">
    <thead>
        <tr>
            <th class="px-4 py-2">Track</th>
            <th class="px-4 py-2">Artist</th>
            <th class="px-4 py-2">Rating</th>
            <th class="px-4 py-2">Total Matches</th>
        </tr>
    </thead>
    <tbody>
        {#each leaderboard as song}
            <tr>
                <td class="border px-4 py-2">{song.name}</td>
                <td class="border px-4 py-2"
                    >{song.artists.map((artist) => artist.name).join(", ")}</td
                >
                <td class="border px-4 py-2"
                    >{song.rating.toFixed(2)} (±{song.deviation.toFixed(2)})</td
                >
                <td class="border px-4 py-2">{song.total_matches}</td>
            </tr>
        {/each}
    </tbody>
</table>

<!-- Matches -->
{#if match}
    <h2 class="text-lg font-bold mt-4">Match</h2>
    <div class="flex">
        <button
            type="button"
            class="border p-4 mr-4 cursor-pointer text-left"
            on:click={() => choose_winner(match!.song_a.id)}
        >
            <h3 class="font-bold">{match.song_a.name}</h3>
            <img
                src={match.song_a.image_url}
                alt={match.song_a.name}
                class="w-32 h-32 object-cover"
            />
            <p>
                Rating: {match.song_a.rating.toFixed(2)} (±{match.song_a.deviation.toFixed(
                    2,
                )})
            </p>
            <p>Total Matches: {match.song_a.total_matches}</p>
        </button>
        <button
            type="button"
            class="border p-4 cursor-pointer text-left"
            on:click={() => choose_winner(match!.song_b.id)}
        >
            <h3 class="font-bold">{match.song_b.name}</h3>
            <img
                src={match.song_b.image_url}
                alt={match.song_b.name}
                class="w-32 h-32 object-cover"
            />
            <p>
                Rating: {match.song_b.rating.toFixed(2)} (±{match.song_b.deviation.toFixed(
                    2,
                )})
            </p>
            <p>Total Matches: {match.song_b.total_matches}</p>
        </button>
    </div>
{/if}
