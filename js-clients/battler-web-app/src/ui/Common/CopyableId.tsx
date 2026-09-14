import { useState } from "react";
import styles from "./CopyableId.module.scss";

interface CopyableIdProps {
  id: string;
  type: "battle" | "replay" | "proposal";
}

export default function CopyableId({ id, type }: CopyableIdProps) {
  const [copied, setCopied] = useState(false);

  const getTargetUrl = () => {
    const base = import.meta.env.BASE_URL || "/";
    const baseNoTrailing = base.endsWith("/") ? base.slice(0, -1) : base;
    const cleanPath = `/${baseNoTrailing}/${type}/${id}`.replace(/\/+/g, "/");
    return `${window.location.origin}${cleanPath}`;
  };

  const handleCopy = (e: React.MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault();
    const url = getTargetUrl();
    navigator.clipboard
      .writeText(url)
      .then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      })
      .catch((err) => {
        console.error("Failed to copy URL to clipboard: ", err);
      });
  };

  const url = getTargetUrl();

  return (
    <span className={styles.container}>
      <a
        href={url}
        onClick={handleCopy}
        className={`${styles.idLink} ${copied ? styles.copied : ""}`}
        title="Click to copy page URL"
      >
        <span className={styles.idText}>{id}</span>
        {copied && <span className={styles.checkmark}>✓</span>}
      </a>
    </span>
  );
}
