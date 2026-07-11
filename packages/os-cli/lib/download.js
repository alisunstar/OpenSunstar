import fs from "node:fs";
import crypto from "node:crypto";
import https from "node:https";
import path from "node:path";

function followRedirect(url, redirects = 0) {
  return new Promise((resolve, reject) => {
    if (redirects > 5) {
      reject(new Error("Too many redirects while downloading os binary"));
      return;
    }

    https
      .get(url, { headers: { "User-Agent": "opensunstar-os-npm" } }, (res) => {
        if (
          res.statusCode &&
          res.statusCode >= 300 &&
          res.statusCode < 400 &&
          res.headers.location
        ) {
          followRedirect(res.headers.location, redirects + 1)
            .then(resolve)
            .catch(reject);
          return;
        }

        if (res.statusCode !== 200) {
          reject(
            new Error(
              `Download failed (${res.statusCode}): ${url}\nHint: check that the GitHub Release includes this asset.`,
            ),
          );
          res.resume();
          return;
        }

        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

export async function downloadToFile(
  url,
  destPath,
  { expectedSha256, artifactName } = {},
) {
  fs.mkdirSync(path.dirname(destPath), { recursive: true });
  const data = await followRedirect(url);

  if (expectedSha256) {
    const actualSha256 = crypto.createHash("sha256").update(data).digest("hex");
    const expected = expectedSha256.toLowerCase();

    if (actualSha256 !== expected) {
      throw new Error(
        `SHA256 mismatch for ${
          artifactName ?? path.basename(destPath)
        }: expected ${expected}, got ${actualSha256}`,
      );
    }
  }

  fs.writeFileSync(destPath, data);
}
