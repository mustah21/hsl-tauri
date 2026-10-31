import { useState, useCallback, useRef, useEffect } from "react";
import { forwardGeocode, reverseGeocode } from "../services/hslService";

export function useGeocoding() {
  const [fromLoc, setFromLoc] = useState(null);
  const [toLoc, setToLoc] = useState(null);
  const [fromInput, setFromInput] = useState("");
  const [toInput, setToInput] = useState("");
  const [fromError, setFromError] = useState(false);
  const [toError, setToError] = useState(false);

  const lastGeocodedFrom = useRef("");
  const lastGeocodedTo = useRef("");
  const isGeocodingFrom = useRef(false);
  const isGeocodingTo = useRef(false);
  const fromLocRef = useRef(null);
  const toLocRef = useRef(null);

  const geocodeAddress = async (address) => {
    try {
      return await forwardGeocode(address);
    } catch (err) {
      console.error("Geocoding failed:", err);
    }
    return null;
  };

  const handleSetFrom = useCallback(async (lat, lon) => {
    if (typeof lat !== "number" || typeof lon !== "number" || isNaN(lat) || isNaN(lon)) {
      console.error("Invalid coordinates:", lat, lon);
      return;
    }
    let address = "Selected Location";
    try {
      address = await reverseGeocode(lat, lon);
    } catch (err) {
      console.error("Reverse geocoding failed:", err);
    }
    setFromLoc({ lat, lon, name: address });
    setFromInput(address);
    lastGeocodedFrom.current = address;
    fromLocRef.current = { lat, lon, name: address };
    setFromError(false);
  }, []);

  const handleSetTo = useCallback(async (lat, lon) => {
    if (typeof lat !== "number" || typeof lon !== "number" || isNaN(lat) || isNaN(lon)) {
      console.error("Invalid coordinates:", lat, lon);
      return;
    }
    let address = "Selected Location";
    try {
      address = await reverseGeocode(lat, lon);
    } catch (err) {
      console.error("Reverse geocoding failed:", err);
    }
    setToLoc({ lat, lon, name: address });
    setToInput(address);
    lastGeocodedTo.current = address;
    toLocRef.current = { lat, lon, name: address };
    setToError(false);
  }, []);

  const swapLocations = useCallback(() => {
    const prevFrom = fromLocRef.current;
    const prevTo = toLocRef.current;
    setFromLoc(prevTo ?? null);
    setToLoc(prevFrom ?? null);
    fromLocRef.current = prevTo ?? null;
    toLocRef.current = prevFrom ?? null;
    setFromInput(prevTo?.name ?? "");
    setToInput(prevFrom?.name ?? "");
    lastGeocodedFrom.current = prevTo?.name ?? "";
    lastGeocodedTo.current = prevFrom?.name ?? "";
    setFromError(false);
    setToError(false);
  }, []);

  // Debounced forward-geocode on fromInput change (2 s delay)
  useEffect(() => {
    const trimmedInput = fromInput.trim();
    if (!trimmedInput || trimmedInput === lastGeocodedFrom.current) return;

    const timer = setTimeout(async () => {
      if (isGeocodingFrom.current) return;
      isGeocodingFrom.current = true;
      const searchTerm = fromInput;
      const result = await geocodeAddress(searchTerm);
      isGeocodingFrom.current = false;

      if (result) {
        setFromLoc(result);
        fromLocRef.current = result;
        if (fromInput === searchTerm) {
          setFromInput(result.name);
          lastGeocodedFrom.current = result.name;
        }
        setFromError(false);
      } else {
        setFromError(true);
        console.warn("Address not found:", searchTerm);
      }
    }, 2000);

    return () => clearTimeout(timer);
  }, [fromInput]);

  // Debounced forward-geocode on toInput change (2 s delay)
  useEffect(() => {
    const trimmedInput = toInput.trim();
    if (!trimmedInput || trimmedInput === lastGeocodedTo.current) return;

    const timer = setTimeout(async () => {
      if (isGeocodingTo.current) return;
      isGeocodingTo.current = true;
      const searchTerm = toInput;
      const result = await geocodeAddress(searchTerm);
      isGeocodingTo.current = false;

      if (result) {
        setToLoc(result);
        toLocRef.current = result;
        if (toInput === searchTerm) {
          setToInput(result.name);
          lastGeocodedTo.current = result.name;
        }
        setToError(false);
      } else {
        setToError(true);
        console.warn("Address not found:", searchTerm);
      }
    }, 2000);

    return () => clearTimeout(timer);
  }, [toInput]);

  return {
    fromLoc,
    toLoc,
    fromInput,
    setFromInput,
    toInput,
    setToInput,
    fromError,
    setFromError,
    toError,
    setToError,
    handleSetFrom,
    handleSetTo,
    swapLocations,
  };
}
