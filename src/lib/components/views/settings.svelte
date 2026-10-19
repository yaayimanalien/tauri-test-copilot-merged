<script lang="ts">
  import { toast } from "svelte-sonner";
  import Moon from "@lucide/svelte/icons/moon";
  import Sun from "@lucide/svelte/icons/sun";
  import Monitor from "@lucide/svelte/icons/monitor";
  import Lock from "@lucide/svelte/icons/lock";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Check from "@lucide/svelte/icons/check";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Switch } from "$lib/components/ui/switch";
  import ConfirmDialog from "$lib/components/confirm-dialog.svelte";
  import { journal } from "$lib/journal.svelte";
  import { api, errMsg } from "$lib/api";
  import { ACCENTS, appearance, setAppearance, type Accent, type Font, type Theme } from "$lib/appearance.svelte";

  let { onlock }: { onlock: () => void } = $props();

  const themes: { id: Theme; label: string; icon: typeof Moon }[] = [
    { id: "dark", label: "Dark", icon: Moon },
    { id: "light", label: "Light", icon: Sun },
    { id: "system", label: "Auto", icon: Monitor },
  ];
  const fonts: { id: Font; label: string }[] = [
    { id: "system", label: "Sans" },
    { id: "serif", label: "Serif" },
    { id: "mono", label: "Mono" },
  ];
  // swatch colours mirror the --brand-h values in app.css
  const hue: Record<Accent, number> = { blue: 255, purple: 295, pink: 355, red: 27, orange: 55, green: 150 };

  const st = $derived(journal.status);
  let biometrics = $state(false);
  $effect(() => { biometrics = !!st?.biometricEnabled; });
  let pwOpen = $state(false);
  let curPw = $state("");
  let newPw = $state("");
  let cid = $state("");
  let csec = $state("");
  let connecting = $state(false);
  let disconnectOpen = $state(false);
  const driveConnected = $derived(!!st?.driveConnected);

  const ago = (ts: number) => {
    const s = Math.max(0, Math.round(Date.now() / 1000 - ts));
    if (s < 60) return "just now";
    if (s < 3600) return `${Math.round(s / 60)} min ago`;
    if (s < 86400) return `${Math.round(s / 3600)} h ago`;
    return new Date(ts * 1000).toLocaleDateString();
  };

  async function toggleBiometric(on: boolean) {
    try {
      await api.setBiometric(on);
      toast(on ? "Biometric unlock is on" : "Biometric unlock is off");
    } catch (e) {
      biometrics = !on;
      toast.error(errMsg(e));
    }
    await journal.refreshStatus().catch(() => {});
  }

  async function changePw() {
    try {
      await api.changePassword(curPw, newPw);
      curPw = newPw = "";
      pwOpen = false;
      toast("Password updated");
      journal.scheduleSync();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }

  async function connect() {
    connecting = true;
    try {
      await api.driveConnect(cid, csec);
      csec = "";
      await journal.refreshStatus();
      toast("Google Drive connected");
      journal.sync(true);
    } catch (e) {
      toast.error(errMsg(e));
    } finally {
      connecting = false;
    }
  }

  async function disconnect() {
    try {
      await api.driveDisconnect();
      await journal.refreshStatus();
    } catch (e) {
      toast.error(errMsg(e));
    }
  }
</script>

{#snippet segmented(options: { id: string; label: string; icon?: typeof Moon }[], value: string, set: (v: any) => void, name: string)}
  <div class="flex rounded-[0.65rem] bg-muted p-0.5" role="radiogroup" aria-label={name}>
    {#each options as o (o.id)}
      <button
        type="button"
        role="radio"
        aria-checked={value === o.id}
        onclick={() => set(o.id)}
        class="flex min-w-14 cursor-pointer items-center justify-center gap-1.5 rounded-[0.5rem] px-3 py-1.5 text-[0.8125rem] font-medium transition-all outline-none focus-visible:ring-2 focus-visible:ring-ring
               {value === o.id ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}"
      >
        {#if o.icon}<o.icon class="size-3.5" />{/if}{o.label}
      </button>
    {/each}
  </div>
{/snippet}

<div class="page">
  <h1 class="large-title">Settings</h1>

  <div class="space-y-8">
    <section>
      <h2 class="set-label">Appearance</h2>
      <div class="set-group">
        <div class="set-row">
          <span>Theme</span>
          {@render segmented(themes, appearance.theme, (v) => setAppearance({ theme: v }), "Theme")}
        </div>
        <div class="set-row">
          <span>Accent</span>
          <div class="flex gap-2" role="radiogroup" aria-label="Accent colour">
            {#each ACCENTS as a (a)}
              <button
                type="button"
                role="radio"
                aria-checked={appearance.accent === a}
                aria-label={a}
                onclick={() => setAppearance({ accent: a })}
                class="grid size-7 cursor-pointer place-items-center rounded-full text-white outline-none ring-offset-2 ring-offset-card transition-shadow focus-visible:ring-2 focus-visible:ring-ring
                       {appearance.accent === a ? 'ring-2 ring-foreground/70' : ''}"
                style="background: oklch(0.68 0.19 {hue[a]});"
              >
                {#if appearance.accent === a}<Check class="size-3.5" strokeWidth={3} />{/if}
              </button>
            {/each}
          </div>
        </div>
        <div class="set-row">
          <span>Writing font</span>
          {@render segmented(fonts, appearance.font, (v) => setAppearance({ font: v }), "Writing font")}
        </div>
      </div>
    </section>

    <section>
      <h2 class="set-label">Security</h2>
      <div class="set-group">
        {#if st?.biometricAvailable}
        <div class="set-row">
          <div>
            <div>Unlock with biometrics</div>
            <div class="text-[0.8125rem] text-muted-foreground">Use Face ID, Touch ID or Windows Hello</div>
          </div>
          <Switch bind:checked={biometrics} aria-label="Unlock with biometrics" onCheckedChange={toggleBiometric} />
        </div>
        {/if}
        <button type="button" class="set-row" onclick={() => onlock()}>
          <span class="flex items-center gap-2.5"><Lock class="size-4 text-muted-foreground" />Lock now</span>
          <span class="text-[0.8125rem] text-muted-foreground">Ctrl L</span>
        </button>
        <button type="button" class="set-row" onclick={() => (pwOpen = !pwOpen)} aria-expanded={pwOpen}>
          <span>Change password</span>
          <ChevronRight class="size-4 text-muted-foreground transition-transform {pwOpen ? 'rotate-90' : ''}" />
        </button>
        {#if pwOpen}
          <div class="space-y-2.5 p-4">
            <Input type="password" bind:value={curPw} placeholder="Current password" autocomplete="current-password" />
            <Input type="password" bind:value={newPw} placeholder="New password (8+ characters)" autocomplete="new-password" />
            <Button class="w-full" onclick={changePw}>Update password</Button>
          </div>
        {/if}
      </div>
      <p class="set-note">Entries are encrypted with a key that only your password (or biometrics) can unlock. If you forget the password, nobody, including Daylight, can recover your entries.</p>
    </section>

    <section>
      <h2 class="set-label">Backup</h2>
      <div class="set-group">
        {#if !driveConnected}
          <div class="space-y-2.5 p-4">
            <Input bind:value={cid} placeholder="Google client ID" spellcheck="false" />
            <Input type="password" bind:value={csec} placeholder="Google client secret" spellcheck="false" />
            <Button class="w-full" disabled={connecting} onclick={connect}>{connecting ? "Waiting for Google…" : "Connect Google Drive"}</Button>
          </div>
        {:else}
          <div class="set-row"><span>Google Drive</span><span class="text-[0.8125rem] text-muted-foreground">{journal.syncError ? "Sync failed" : st?.lastBackup ? "Synced " + ago(st.lastBackup) : "Not synced yet"}</span></div>
          <button type="button" class="set-row text-brand" disabled={journal.syncing} onclick={() => journal.sync(false)}>{journal.syncing ? "Syncing…" : "Sync now"}</button>
          {#if journal.syncError}<p class="px-4 pb-3 text-[0.8125rem] text-destructive">{journal.syncError}</p>{/if}
          <button type="button" class="set-row text-destructive" onclick={() => (disconnectOpen = true)}>Disconnect</button>
        {/if}
      </div>
      <p class="set-note">Only new and changed entries are uploaded, already encrypted. Google only sees scrambled files in a hidden app folder.</p>
    </section>

    <section>
      <h2 class="set-label">About</h2>
      <div class="set-group">
        <div class="set-row"><span>Version</span><span class="text-[0.8125rem] text-muted-foreground">0.1.0</span></div>
      </div>
    </section>
  </div>
</div>

<ConfirmDialog
  bind:open={disconnectOpen}
  title="Disconnect Google Drive?"
  description="Existing backups stay in your Drive."
  confirmLabel="Disconnect"
  onconfirm={disconnect}
/>
