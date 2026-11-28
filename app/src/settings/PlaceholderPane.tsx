/** A Settings section for a feature that is not built yet. */
export function PlaceholderPane({ title }: { title: string }) {
  return (
    <>
      <h1 className="m-0 text-xl font-semibold">{title}</h1>
      <p className="m-0" style={{ color: "var(--muted)" }}>
        Not built yet. Settings for this appear when the feature lands.
      </p>
    </>
  );
}
