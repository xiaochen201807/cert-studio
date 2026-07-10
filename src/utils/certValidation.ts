export interface CertRequestValidationResult {
  commonName: string;
  dnsNames: string[];
  ipAddresses: string[];
  pfxPassword: string;
  error: string | null;
}

export const parseCommaSeparatedValues = (value: string): string[] =>
  value
    .split(/[,\uFF0C]/)
    .map((item) => item.trim().toLowerCase())
    .filter((item) => item.length > 0);

export const isValidDnsName = (value: string, allowWildcard: boolean): boolean => {
  const candidate = allowWildcard && value.startsWith("*.") ? value.slice(2) : value;
  if (!candidate || candidate.length > 253 || !/^[\x00-\x7F]+$/.test(candidate) || candidate.endsWith(".")) {
    return false;
  }

  return candidate.split(".").every((label) =>
    label.length > 0
    && label.length <= 63
    && !label.startsWith("-")
    && !label.endsWith("-")
    && /^[a-z0-9-]+$/i.test(label)
  );
};

export const validateCertificateRequest = (input: {
  commonName: string;
  dnsInput: string;
  ipInput: string;
  pfxPassword: string;
  days: number;
}): CertRequestValidationResult => {
  const commonName = input.commonName.trim().toLowerCase();
  const dnsNames = parseCommaSeparatedValues(input.dnsInput);
  const ipAddresses = parseCommaSeparatedValues(input.ipInput);
  const pfxPassword = input.pfxPassword.trim();

  const result = { commonName, dnsNames, ipAddresses, pfxPassword };

  if (!isValidDnsName(commonName, false)) {
    return { ...result, error: "常用名称 (Common Name) 必须是有效且不含通配符的 DNS 名称。" };
  }
  if (dnsNames.length === 0 || dnsNames.some((name) => !isValidDnsName(name, true))) {
    return { ...result, error: "请至少填写一个有效的 DNS 使用者备用名称。" };
  }
  if (!dnsNames.includes(commonName)) {
    return { ...result, error: "常用名称 (Common Name) 必须同时包含在 DNS SAN 列表中。" };
  }
  if (ipAddresses.some((ip) => ip.includes("*"))) {
    return { ...result, error: "IP 使用者备用名称 (IP SANs) 不支持通配符！" };
  }
  if (!Number.isInteger(input.days) || input.days < 1 || input.days > 825) {
    return { ...result, error: "服务端证书有效期必须在 1 到 825 天之间。" };
  }
  if (pfxPassword.length < 8) {
    return { ...result, error: "PFX/PKCS#12 导出密码至少需要 8 个字符。" };
  }

  return { ...result, error: null };
};
