<script lang="ts">
  import Lock from "@lucide/svelte/icons/lock";
  import Fingerprint from "@lucide/svelte/icons/fingerprint";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";

  import { onMount } from "svelte";
  import { api, errMsg } from "$lib/api";

  let {
    mode = "unlock",
    notice = "",
    onunlock,
  }: { mode?: "unlock" | "setup"; notice?: string; onunlock: () => void | Promise<void> } = $props();

  const setup = $derived(mode === "setup");
  let pw = $state("");
  let pw2 = $state("");
  let error = $state("");
  let busy = $state(false);
  let restoreOpen = $state(false);
  let cid = $state("");
  let csec = $state("");
  let bio = $state(false);

  onMount(async () => {
    try {
      const st = await api.status();
      bio = st.biometricEnabled;
      if (bio && !notice && !setup) biometric();
    } catch {}
  });

  async function run(fn: () => Promise<void>) {
    busy = true;
    error = "";
    try {
      await fn();
      pw = pw2 = csec = "";
      await onunlock();
    } catch (e) {
      error = errMsg(e);
    } finally {
      busy = false;
    }
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!pw || busy) return;
    if (setup) {
      if (pw.length < 8) return void (error = "Use at least 8 characters");
      if (pw !== pw2) return void (error = "Passwords don't match");
    }
    run(() => (setup ? api.setup(pw) : api.unlock(pw)));
  }

  const biometric = () => !busy && run(() => api.unlockBiometric());
  function restore() {
    if (!pw || !cid.trim() || !csec.trim()) return void (error = "Enter your password and both Google values");
    run(async () => void (await api.restore(cid, csec, pw)));
  }
</script>

<main class="grid min-h-dvh place-items-center px-6 py-10">
  <form class="w-full max-w-sm space-y-6" onsubmit={submit} autocomplete="off">
    <div class="space-y-3 text-center">
      <div class="glass mx-auto grid size-16 place-items-center rounded-[1.4rem]">
        <Lock class="size-7" strokeWidth={1.7} />
      </div>
      <h1 class="text-[1.75rem] font-semibold tracking-tight">{setup ? "Welcome to Daylight" : "Daylight"}</h1>
      <p class="mx-auto max-w-[19rem] text-sm text-balance text-muted-foreground">
        {setup
          ? "Choose a password. It encrypts your diary and can't be recovered if you forget it."
          : "Enter your password to open your diary."}
      </p>
    </div>

    <div class="space-y-3">
      <Input
        type="password"
        bind:value={pw}
        placeholder={setup ? "New password (8+ characters)" : "Password"}
        autocomplete={setup ? "new-password" : "current-password"}
        aria-invalid={!!error}
      />
      {#if setup}
        <Input type="password" bind:value={pw2} placeholder="Confirm password" autocomplete="new-password" aria-invalid={!!error} />
      {/if}
      {#if error}<p role="alert" class="text-sm text-destructive">{error}</p>{:else if notice}<p class="text-sm text-muted-foreground">{notice}</p>{/if}
    </div>

    <div class="space-y-2.5">
      <Button type="submit" size="lg" class="w-full" disabled={busy}>{setup ? "Create vault" : "Unlock"}</Button>
      {#if !setup && bio}
        <Button type="button" variant="secondary" size="lg" class="w-full" disabled={busy} onclick={biometric}>
          <Fingerprint /> Unlock with biometrics
        </Button>
      {/if}
    </div>

    {#if setup}
      <div class="space-y-3 text-center">
        {#if !restoreOpen}
          <Button type="button" variant="link" size="sm" onclick={() => (restoreOpen = true)}>Restore from Google Drive</Button>
        {:else}
          <div class="space-y-3 rounded-2xl border bg-card/50 p-4 text-left">
            <p class="text-[0.8125rem] text-muted-foreground">
              Enter the password from your other device, plus your Google OAuth client.
            </p>
            <Input bind:value={cid} placeholder="Google client ID" spellcheck="false" />
            <Input type="password" bind:value={csec} placeholder="Google client secret" spellcheck="false" />
            <Button type="button" class="w-full" disabled={busy} onclick={restore}>Restore from Drive</Button>
          </div>
        {/if}
      </div>
    {/if}
  </form>
</main>
