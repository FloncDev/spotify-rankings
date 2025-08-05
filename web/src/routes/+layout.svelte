<script lang="ts">
	import "../app.css";
	import { onMount } from "svelte";
	import { isLoggedIn } from "$lib/stores";

	onMount(async () => {
		let logged_in = await fetch("http://localhost:3000/me", {
			credentials: "include",
		})
			.then((response) => {
				if (response.ok) {
					return true;
				} else {
					return false;
				}
			})
			.catch(() => false);

		isLoggedIn.set(logged_in);
	});

	let { children } = $props();
</script>

{@render children()}
