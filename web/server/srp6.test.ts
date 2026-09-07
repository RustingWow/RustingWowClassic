import assert from "node:assert/strict";
import { test } from "node:test";

import { calculateVerifier } from "./srp6.ts";

test("matches wow_srp password verifier for a known salt", () => {
  const salt = Buffer.from(
    "AFE5D28E925DBB3DAFED5D91ACA0928940E8FBFEF2D2A3CC154ADA0FE6ABEF6F",
    "hex",
  ).reverse();
  const expected = Buffer.from(
    "21B4153B0A938D0A69D28F2690CC3F79A99A13C40CACB525B3B79D4201EB33FF",
    "hex",
  ).reverse();

  const verifier = calculateVerifier(
    "LF2BGFQIFQ3HZ1ZF",
    "MVRVMUJFWRA0IBVK",
    salt,
  );
  assert.equal(verifier.toString("hex"), expected.toString("hex"));
});
