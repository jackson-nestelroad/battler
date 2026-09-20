import Modal from "../Modal/Modal";

import styles from "./AboutModal.module.scss";

export interface AboutModalProps {
  isOpen: boolean;
  onClose: () => void;
}

const TECH_STACK = [
  { name: "Rust", icon: "/assets/icons/rust.svg" },
  { name: "WebAssembly", icon: "/assets/icons/webassembly.svg" },
  { name: "React", icon: "/assets/icons/react.svg" },
  { name: "Vite", icon: "/assets/icons/vite.svg" },
  { name: "Google Gemini", icon: "/assets/icons/googlegemini.svg" },
  { name: ".NET", icon: "/assets/icons/dotnet.svg" },
];

export default function AboutModal({ isOpen, onClose }: AboutModalProps) {
  const currentYear = new Date().getFullYear();

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="About" maxWidth="sm">
      <div className={styles.aboutContainer}>
        {/* Brand Hero */}
        <div className={styles.hero}>
          <img src="/logo.svg" alt="Battler" className={styles.logo} />
          <h3 className={styles.title}>Battler</h3>
          <p className={styles.tagline}>Pokémon battle engine</p>
        </div>

        {/* Engine Attribution */}
        <p className={styles.description}>
          Powered by{" "}
          <a
            href="https://battler.rs"
            target="_blank"
            rel="noopener noreferrer"
            className={styles.engineLink}
          >
            battler.rs
          </a>
          .
        </p>

        {/* Tech Stack Pills */}
        <div className={styles.techSection}>
          <span className={styles.techLabel}>Built With</span>
          <div className={styles.techPills}>
            {TECH_STACK.map((tech) => (
              <div key={tech.name} className={styles.techPill}>
                <img src={tech.icon} alt="" width={14} height={14} />
                <span>{tech.name}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Minimal Footer */}
        <div className={styles.footer}>
          <div className={styles.projectMeta}>
            <span className={styles.copyright}>© {currentYear} Jackson Nestelroad</span>
            <span className={styles.metaSeparator}>•</span>
            <a
              href="https://github.com/jackson-nestelroad/battler"
              target="_blank"
              rel="noopener noreferrer"
              className={styles.githubLink}
            >
              <img src="/assets/icons/github.svg" alt="" width={12} height={12} />
              <span>GitHub</span>
            </a>
          </div>
          <p className={styles.disclaimer}>
            Pokémon is © 1995–{currentYear} Nintendo, Creatures, and Game Freak. Battler is an
            unofficial fan project and is not affiliated with Nintendo or The Pokémon Company.
          </p>
        </div>
      </div>
    </Modal>
  );
}
