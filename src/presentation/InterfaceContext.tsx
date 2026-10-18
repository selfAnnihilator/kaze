import { createContext, useContext } from "react";

export type InterfaceStyle = "spatial" | "brutalist";
export const InterfaceContext = createContext<InterfaceStyle>("spatial");
export const useInterfaceStyle = () => useContext(InterfaceContext);
