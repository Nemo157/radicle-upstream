<script>
  import { status as store, StatusType } from "../../src/localPeer.ts";

  import Remote from "../Component/Remote.svelte";
  import { Icon } from "../Primitive";
  import Tooltip from "./Tooltip.svelte";

  import Syncing from "./ConnectionStatusIndicator/Syncing.svelte";
  import Offline from "./ConnectionStatusIndicator/Offline.svelte";

  const peerCount = count => {
    if (count === 1) {
      return "1 peer";
    } else {
      return `${count} peers`;
    }
  };
</script>

<style>
  .item {
    width: var(--sidebar-width);
    height: 32px;
    margin-bottom: 16px;
    position: relative;
    display: flex;
    justify-content: center;
    align-items: center;
    cursor: pointer;
  }

  .indicator:hover:before {
    position: absolute;
    content: "";
    width: 4px;
    height: 32px;
    background-color: var(--color-foreground-level-5);
    top: 0px;
    left: 0px;
    border-top-right-radius: 4px;
    border-bottom-right-radius: 4px;
  }

  .indicator :global(div:hover svg) {
    fill: var(--color-secondary);
  }
</style>

<Remote {store} let:data={status}>
  <div>
    {#if status.type === StatusType.Online}
      <Tooltip value={`You’re connected to ${peerCount(status.connected)}`}>
        <div class="item indicator" data-cy="connection-status">
          <Icon.Network />
        </div>
      </Tooltip>
    {:else if status.type === StatusType.Syncing}
      <Tooltip
        value={`Syncing with ${peerCount(status.syncs)} to get new content from your network`}>
        <div class="item indicator" data-cy="connection-status">
          <Syncing />
        </div>
      </Tooltip>
    {:else if status.type === StatusType.Offline || status.type === StatusType.Started}
      <Tooltip value="You’re not connected to any peers">
        <div class="item indicator" data-cy="connection-status">
          <Offline />
        </div>
      </Tooltip>
    {:else if status.type === StatusType.Stopped}
      <Tooltip value="The app couldn't start your peer">
        <div class="item indicator" data-cy="connection-status">
          <Offline style="fill: var(--color-negative);" />
        </div>
      </Tooltip>
    {/if}
  </div>
</Remote>
