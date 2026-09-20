import React, { useCallback, useEffect, useMemo, useState } from "react";
import type { BugReportReactCrash, BugReportResponse } from "../../../core/bugReport";
import {
  downloadDiagnosticJson,
  gatherBugReportPayload,
  getGitHubFallbackUrl,
  submitBugReport,
} from "../../../core/bugReport";
import { useAppSelector } from "../../../store/store";
import { getBattleTurnNumber } from "../../../utils/battleState";
import Modal from "../Modal/Modal";

import styles from "./BugReportModal.module.scss";

export interface BugReportModalProps {
  isOpen: boolean;
  onClose: () => void;
  reactCrash?: BugReportReactCrash;
}

export default function BugReportModal({
  isOpen,
  onClose,
  reactCrash,
}: BugReportModalProps) {
  const [title, setTitle] = useState(reactCrash ? `Crash: ${reactCrash.message}` : "");
  const [description, setDescription] = useState("");
  const [showPreview, setShowPreview] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<BugReportResponse | null>(null);

  const resetForm = useCallback(() => {
    setTitle(reactCrash ? `Crash: ${reactCrash.message}` : "");
    setDescription("");
    setShowPreview(false);
    setError(null);
    setResult(null);
  }, [reactCrash]);

  // Reset form inputs whenever modal is opened
  useEffect(() => {
    if (isOpen) {
      resetForm();
    }
  }, [isOpen, resetForm]);

  // Subscribe to live Redux state so opening or keeping the modal open always reflects current view/battle
  const state = useAppSelector((s) => s);

  const diagnosticPayload = useMemo(() => {
    return gatherBugReportPayload(
      title.trim() || "Untitled Bug Report",
      description.trim() || "No description provided",
      reactCrash,
      state,
    );
  }, [title, description, reactCrash, state]);

  const handleClose = () => {
    if (isSubmitting) return;
    resetForm();
    onClose();
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !description.trim() || isSubmitting) return;

    setIsSubmitting(true);
    setError(null);

    try {
      const response = await submitBugReport(diagnosticPayload);
      setResult(response);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : "Failed to connect to bug reporting service.";
      setError(msg);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleExportJson = () => {
    downloadDiagnosticJson(diagnosticPayload);
  };

  const handleFallbackToGitHub = () => {
    downloadDiagnosticJson(diagnosticPayload);
    const url = getGitHubFallbackUrl(diagnosticPayload);
    window.open(url, "_blank", "noopener,noreferrer");
  };

  const activeBattle = diagnosticPayload.battleDebug;
  const contextDescription = activeBattle
    ? `Battle ${activeBattle.battleId.slice(0, 8)} (Turn ${getBattleTurnNumber(activeBattle)})`
    : diagnosticPayload.view.charAt(0).toUpperCase() + diagnosticPayload.view.slice(1);

  return (
    <Modal isOpen={isOpen} onClose={handleClose} title="Report Bug" maxWidth="sm">
      {result ? (
        <div className={`${styles.successState} flex-col align-center text-center gap-m`}>
          <div className={styles.successIcon} aria-hidden="true">
            ✓
          </div>
          <h3 className={styles.successTitle}>Bug report submitted!</h3>
          {result.softLaunch && (
            <span className="badge badge-secondary">
              Soft-launch mode • Saved locally
            </span>
          )}
          {result.dryRun && (
            <span className={styles.dryRunNotice}>{result.message}</span>
          )}
          <div className="flex-row align-center gap-s mt-xs">
            <a
              href={result.issueUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="btn btn-primary btn-sm"
            >
              {result.softLaunch ? "View local report ↗" : `View issue #${result.issueNumber} ↗`}
            </a>
            <button type="button" className="btn btn-secondary btn-sm" onClick={handleClose}>
              Close
            </button>
          </div>
        </div>
      ) : (
        <form onSubmit={handleSubmit} className={`${styles.modalBody} flex-col gap-l`}>
          <div className="form-group">
            <label htmlFor="bug-title">
              Title
            </label>
            <input
              id="bug-title"
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Brief summary of the issue"
              disabled={isSubmitting}
              required
            />
          </div>

          <div className="form-group">
            <label htmlFor="bug-description">
              Description
            </label>
            <textarea
              id="bug-description"
              className={styles.textArea}
              rows={4}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="What happened and steps to reproduce..."
              disabled={isSubmitting}
              required
            />
          </div>

          <div className={styles.contextCard}>
            <div className="flex-row align-center justify-between gap-s">
              <div className="flex-row align-center gap-xs min-w-0">
                <span className={styles.contextDot} aria-hidden="true" />
                <span className={styles.contextLabel}>Context:</span>
                <span className={styles.contextValue} title={contextDescription}>
                  {contextDescription}
                </span>
              </div>
              <button
                type="button"
                className="btn btn-secondary btn-sm"
                onClick={() => setShowPreview(!showPreview)}
                aria-expanded={showPreview}
              >
                {showPreview ? "Hide data" : "View data"}
              </button>
            </div>

            {showPreview && (
              <div className={styles.previewContainer}>
                <pre className={styles.previewJson}>
                  {JSON.stringify(diagnosticPayload, null, 2)}
                </pre>
              </div>
            )}
          </div>

          {error && (
            <div className="alert alert-danger flex-col gap-xs align-start">
              <span>{error}</span>
              <button
                type="button"
                className="btn btn-sm btn-secondary"
                onClick={handleFallbackToGitHub}
              >
                Export JSON & open on GitHub ↗
              </button>
            </div>
          )}

          <div className={`${styles.modalFooter} flex-row align-center justify-between gap-m mt-xs`}>
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              onClick={handleExportJson}
              title="Download diagnostic JSON"
            >
              Export JSON
            </button>
            <div className="flex-row gap-s align-center">
              <button
                type="button"
                className="btn btn-secondary btn-sm"
                onClick={handleClose}
                disabled={isSubmitting}
              >
                Cancel
              </button>
              <button
                type="submit"
                className="btn btn-primary btn-sm"
                disabled={isSubmitting || !title.trim() || !description.trim()}
              >
                {isSubmitting ? "Submitting..." : "Submit"}
              </button>
            </div>
          </div>
        </form>
      )}
    </Modal>
  );
}

