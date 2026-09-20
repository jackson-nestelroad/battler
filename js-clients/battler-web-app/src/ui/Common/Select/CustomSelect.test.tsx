import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import CustomSelect from "./CustomSelect";

describe("CustomSelect", () => {
  const options = [
    { value: "opt1", label: "Option 1" },
    { value: "opt2", label: "Option 2", endContent: <span>Extra</span> },
    { value: "opt3", label: "Option 3", disabled: true },
  ];

  it("renders placeholder and accessible combobox role when unselected", () => {
    const html = renderToStaticMarkup(
      <CustomSelect
        id="test-select"
        value=""
        onChange={() => {}}
        options={options}
        placeholder="Choose something"
      />,
    );

    expect(html).toContain('role="combobox"');
    expect(html).toContain('id="test-select"');
    expect(html).toContain('aria-expanded="false"');
    expect(html).toContain("Choose something");
  });

  it("renders selected option label and endContent inside trigger", () => {
    const html = renderToStaticMarkup(
      <CustomSelect
        id="test-select"
        value="opt2"
        onChange={() => {}}
        options={options}
      />,
    );

    expect(html).toContain("Option 2");
    expect(html).toContain("Extra");
    expect(html).toContain('aria-label="Selected: Option 2"');
  });

  it("disables trigger button when disabled prop is true", () => {
    const html = renderToStaticMarkup(
      <CustomSelect
        value="opt1"
        onChange={() => {}}
        options={options}
        disabled={true}
      />,
    );

    expect(html).toContain("disabled");
  });

  it("includes hidden input with value for form compatibility", () => {
    const html = renderToStaticMarkup(
      <CustomSelect
        id="form-field"
        value="opt1"
        onChange={() => {}}
        options={options}
        required={true}
      />,
    );

    expect(html).toContain('type="hidden"');
    expect(html).toContain('name="form-field"');
    expect(html).toContain('value="opt1"');
    expect(html).toContain("required");
  });
});
