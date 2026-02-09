<script lang="ts">
    import { onMount } from "svelte";
    import type { Song } from "./+page.svelte";

    let { song, offset = $bindable(0) }: { song: Song; offset?: number } =
        $props();
    const max_offset = 150;

    // Dynamically compute the effective multiplier based on whether this card is being swiped
    // If being swiped: use 1 (go in swipe direction)
    // If not being swiped: use -1 (go opposite direction)

    // Smoothly clamp the offset to a maximum of 100 pixels in either direction.
    // e.g. as you get closer to the 100 pixel limit, the offset will increase slower
    let offset_clamped = $derived.by(() => {
        const effective_offset = offset * effective_multiplier;
        const clamped =
            Math.sign(effective_offset) *
            (1 - Math.exp(-Math.abs(effective_offset) / max_offset)) *
            max_offset;
        return clamped;
    });
    let initial_x = 0;
    let on_card = $state(false);

    // // Dynamically compute the effective multiplier based on whether this card is being swiped
    // let effective_multiplier = $derived(on_card ? 1 : offset_multiplier);
    let effective_multiplier = $derived(on_card ? 1 : -1);

    let card_element: HTMLDivElement;
    let card_height = $state(0);

    $effect(() => {
        if (card_element) {
            const updateHeight = () => {
                card_height = card_element.offsetHeight;
            };

            // Update height immediately
            updateHeight();

            // Update height when image loads
            const img = card_element.querySelector("img");
            if (img) {
                img.addEventListener("load", updateHeight);
            }

            // Also use ResizeObserver to catch any size changes
            const observer = new ResizeObserver(updateHeight);
            observer.observe(card_element);

            return () => {
                observer.disconnect();
                if (img) {
                    img.removeEventListener("load", updateHeight);
                }
            };
        }
    });

    function handle_touch(event: TouchEvent) {
        // Get the first touch point
        const touch = event.touches[0];

        if (event.type === "touchstart") {
            initial_x = touch.clientX;
            if (card_element) {
                on_card = card_element.contains(touch.target as Node);
            }
        } else if (event.type === "touchmove" && on_card) {
            offset = touch.clientX - initial_x;
        } else if (event.type === "touchend") {
            // Reset the offset when the touch ends
            offset = 0;
        }
    }

    onMount(() => {
        window.addEventListener("touchstart", handle_touch);
        window.addEventListener("touchmove", handle_touch);
        window.addEventListener("touchend", handle_touch);
    });

    // Gradient opacity is based on the offset, so it will be more opaque as you swipe further
    let left_opacity = $derived.by(() => {
        return Math.max(-offset_clamped / max_offset, 0);
    });
    let right_opacity = $derived.by(() => {
        return Math.max(offset_clamped / max_offset, 0);
    });
</script>

<div>
    <div
        class="fixed left-0 pointer-events-none z-10"
        style="width: 200px; height: {card_height}px; opacity: {left_opacity}; background: radial-gradient(ellipse 100px 250px at left center, rgba(34, 197, 94, 0.6), transparent 70%);"
    ></div>

    <div
        class="fixed right-0 pointer-events-none z-10"
        style="width: 200px; height: {card_height}px; opacity: {right_opacity}; background: radial-gradient(ellipse 100px 250px at right center, rgba(239, 68, 68, 0.6), transparent 70%);"
    ></div>

    <!-- The song card -->
    <div
        class="bg-[#1f1f1f] rounded-xl p-4 mb-4"
        style="transform: translateX({offset_clamped /
            2}px) rotate({offset_clamped /
            25}deg); transition: transform 0.1s ease-out;"
        id="song-card"
        bind:this={card_element}
    >
        <!-- {left_opacity} -->
        <img
            src={song.image_url}
            alt={song.name}
            class="w-max object-cover rounded"
        />
        <h3 class="text-lg font-bold">{song.name}</h3>
        <p>
            Rating: {song.rating.toFixed(2)} (±{song.deviation.toFixed(2)})
        </p>
        <p>Total Matches: {song.total_matches}</p>
    </div>
</div>
