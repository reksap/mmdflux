import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";
import { loadWasmModule } from "./wasm-module";

describe("ER diagram WASM rendering", () => {
  it("renders the playground example and replays MMDS with endpoint cardinality", async () => {
    const wasm = await loadWasmModule();
    // wasm-bindgen accepts bytes in Node without needing browser fetch.
    const bytes = await readFile("src/wasm-pkg/mmdflux_wasm_bg.wasm");
    await (
      wasm.default as unknown as (input: {
        module_or_path: Uint8Array;
      }) => Promise<void>
    )({ module_or_path: bytes });
    const source =
      "erDiagram\nCUSTOMER { int id PK }\nORDER { int customer_id FK }\nCUSTOMER ||--o{ ORDER : places";
    for (const format of ["text", "ascii", "svg", "mmds"]) {
      const output = wasm.render(source, format, "{}");
      expect(output).toContain("CUSTOMER");
      expect(output).toContain("customer_id FK");
      if (format === "ascii") {
        expect(output).toContain("0..*");
        expect([...output].every((char) => char.charCodeAt(0) < 128)).toBe(
          true,
        );
      }
      if (format === "svg") {
        expect(output).toContain('marker-start="url(#er-one)"');
        expect(output).toContain('marker-end="url(#er-zero-or-more)"');
      }
      if (format === "mmds") {
        expect(JSON.parse(output).metadata.diagram_type).toBe("er");
        expect(wasm.render(output, "svg", "{}")).toContain(
          'marker-end="url(#er-zero-or-more)"',
        );
      }
    }
    expect(JSON.parse(wasm.validate(source)).valid).toBe(true);
  });
});
