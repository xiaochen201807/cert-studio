export interface CertRequestValidationResult {
  commonName: string;
  dnsNames: string[];
  ipAddresses: string[];
  pfxPassword: string;
  error: string | null;
}

export const parseCommaSeparatedValues = (value: string): string[] =>
  value
    .split(",")
    .map((item) => item.trim())
    .filter((item) => item.length > 0);

export const validateCertificateRequest = (input: {
  commonName: string;
  dnsInput: string;
  ipInput: string;
  pfxPassword: string;
}): CertRequestValidationResult => {
  const commonName = input.commonName.trim();
  const dnsNames = parseCommaSeparatedValues(input.dnsInput);
  const ipAddresses = parseCommaSeparatedValues(input.ipInput);
  const pfxPassword = input.pfxPassword.trim();

  if (!commonName) {
    return { commonName, dnsNames, ipAddresses, pfxPassword, error: "常用名称 (Common Name) 不能为空。" };
  }
  if (commonName.includes(",") || commonName.includes("，")) {
    return { commonName, dnsNames, ipAddresses, pfxPassword, error: "常用名称 (Common Name) 只能填写一个，不能包含逗号！" };
  }
  if (commonName.includes("*")) {
    return { commonName, dnsNames, ipAddresses, pfxPassword, error: "常用名称 (Common Name) 不支持通配符！" };
  }
  if (dnsNames.length === 0) {
    return { commonName, dnsNames, ipAddresses, pfxPassword, error: "请至少填写一个 DNS 使用者备用名称。" };
  }
  if (ipAddresses.some((ip) => ip.includes("*"))) {
    return { commonName, dnsNames, ipAddresses, pfxPassword, error: "IP 使用者备用名称 (IP SANs) 不支持通配符！" };
  }
  if (!pfxPassword) {
    return { commonName, dnsNames, ipAddresses, pfxPassword, error: "请设置 PFX/PKCS#12 导出密码，用于保护 server.pfx 中的私钥。" };
  }

  return { commonName, dnsNames, ipAddresses, pfxPassword, error: null };
};
