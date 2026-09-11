import { selectResource } from "../../store/battlesSlice";
import { useAppDispatch, useAppSelector } from "../../store/store";
import ResourcesHome from "./ResourcesHome";
import TypeChartScreen from "./TypeChartScreen";

export default function Resources() {
  const dispatch = useAppDispatch();
  const activeResource = useAppSelector((state) => state.battles.activeResource);

  if (activeResource === "type-chart") {
    return (
      <TypeChartScreen
        onBack={() => dispatch(selectResource(null))}
      />
    );
  }

  return (
    <ResourcesHome
      onSelectResource={(resource) => dispatch(selectResource(resource))}
    />
  );
}
