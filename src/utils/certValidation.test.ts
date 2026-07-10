import { describe, expect, it } from "vitest";
import { isValidDnsName, validateCertificateRequest } from "./certValidation";

const validInput = {
  commonName: "api.internal.example.com",
  dnsInput: "api.internal.example.com, localhost",
  ipInput: "127.0.0.1",
  pfxPassword: "strong-password",
  days: 365,
};

describe("certificate request validation", () => {
  it("accepts normal and leading wildcard DNS names", () => {
    expect(isValidDnsName("api.internal.example.com", false)).toBe(true);
    expect(isValidDnsName("*.internal.example.com", true)).toBe(true);
  });

  it("rejects Nginx directive injection", () => {
    const result = validateCertificateRequest({
      ...validInput,
      commonName: "example.com; include /tmp/file",
    });
    expect(result.error).toContain("Common Name");
  });

  it("requires the common name in DNS SAN", () => {
    const result = validateCertificateRequest({
      ...validInput,
      dnsInput: "other.example.com",
    });
    expect(result.error).toContain("DNS SAN");
  });

  it("enforces validity and password bounds", () => {
    expect(validateCertificateRequest({ ...validInput, days: 0 }).error).toContain("825");
    expect(validateCertificateRequest({ ...validInput, pfxPassword: "short" }).error).toContain("8");
  });
});
