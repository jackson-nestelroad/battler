import React, { useRef } from "react";
import ErrorBanner from "./ErrorBanner";
import styles from "./JsonEditor.module.scss";

interface JsonEditorProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  error?: string | null;
  success?: string | null;
  required?: boolean;
}

export default function JsonEditor({
  value,
  onChange,
  placeholder,
  error = null,
  success = null,
  required = false,
}: JsonEditorProps) {
  const lineNumbersRef = useRef<HTMLDivElement>(null);

  const handleScroll = (e: React.UIEvent<HTMLTextAreaElement>) => {
    if (lineNumbersRef.current) {
      lineNumbersRef.current.scrollTop = e.currentTarget.scrollTop;
    }
  };

  const lines = value.split("\n");

  return (
    <div className={styles.editorContainer}>
      <ErrorBanner message={error} />
      {success && <div className="alert alert-success">{success}</div>}

      <div className={styles.textareaWrapper}>
        <div ref={lineNumbersRef} className={styles.lineNumbers}>
          {Array.from({ length: Math.max(lines.length, 1) }, (_, i) => (
            <div key={i} className={styles.lineNumber}>
              {i + 1}
            </div>
          ))}
        </div>
        <textarea
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onScroll={handleScroll}
          placeholder={placeholder}
          className={styles.jsonTextarea}
          spellCheck="false"
          required={required}
        />
      </div>
    </div>
  );
}
