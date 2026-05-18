<script lang="ts">
  let { icon, heading, value, sub }: { icon: string; heading: string; value: number; sub: string } =
    $props();
</script>

<div class="card summary-card">
  <span class="card-icon">{icon}</span>
  <div class="summary-body">
    <div class="summary-heading">{heading}</div>
    <div class="summary-value-row">
      <p class="value">{value}</p>
      <p class="sub">{sub}</p>
    </div>
  </div>
</div>

<style>
  .card {
    background: #fff;
    border: 1px solid #dbe2ea;
    border-radius: 8px;
    padding: 20px 16px;
    text-align: center;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
    min-width: 0;
  }

  .summary-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    text-align: left;
  }

  .card-icon {
    font-size: 1.8rem;
    flex: 0 0 auto;
  }

  .summary-body {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .summary-heading {
    color: #64748b;
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.03em;
  }

  .summary-value-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
  }

  .value {
    margin: 0;
    font-size: 1.8rem;
    line-height: 1.1;
    font-weight: 700;
    color: #2e86c1;
  }

  .sub {
    margin: 0;
    font-size: 0.75rem;
    color: #95a5a6;
  }

  @media (max-width: 760px) {
    .summary-card {
      align-items: flex-start;
    }
  }
</style>
