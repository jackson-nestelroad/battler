import Modal from "../Common/Modal/Modal";
import TypeChartGrid from "../Common/TypeChartGrid";

export interface TypeChartModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function TypeChartModal({ isOpen, onClose }: TypeChartModalProps) {
  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="Type Chart"
      maxWidth="lg"
      ariaLabelledBy="type-chart-modal-title"
    >
      <TypeChartGrid />
    </Modal>
  );
}
