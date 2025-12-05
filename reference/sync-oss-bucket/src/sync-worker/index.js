"use strict";

const OSS = require("ali-oss");
const fs = require("fs");

let destOssClient;

const destinationRegion = process.env.DESTINATION_REGION;
const destinationBucket = process.env.DESTINATION_BUCKET;

exports.initialize = async (context, callback) => {
  destOssClient = new OSS({
    region: `oss-${destinationRegion}`,
    bucket: destinationBucket,
    accessKeyId: context.credentials.accessKeyId,
    accessKeySecret: context.credentials.accessKeySecret,
    stsToken: context.credentials.securityToken,
    internal: true,
  });
  callback(null, "");
};

exports.handler = async (event, context, callback) => {
  const { name, etag, sourceRegion, sourceBucket } = JSON.parse(event);
  try {
    const result = await destOssClient.head(name, {});
    const {
      res: {
        headers: { etag: currentEtag },
      },
    } = result;
    if (currentEtag === etag) {
      console.log(`Object ${name} not changed`);
      callback(null, "");
      return;
    }
  } catch (e) {
    if (e.code !== "NoSuchKey") {
      console.log(e);
      callback(e.code, null);
      return;
    }
  }
  console.log(`Start sync ${name}`);
  const sourceOssClient = new OSS({
    region: `oss-${sourceRegion}`,
    bucket: sourceBucket,
    accessKeyId: context.credentials.accessKeyId,
    accessKeySecret: context.credentials.accessKeySecret,
    stsToken: context.credentials.securityToken,
    internal: sourceRegion === context.region,
  });
  const tempFilePath = `/tmp/${name}`;
  const tempTokens = tempFilePath.split("/");
  tempTokens.pop();
  const tempDir = tempTokens.join("/");
  fs.mkdirSync(tempDir, { recursive: true });
  console.log(`Start downloading ${name}`);
  await sourceOssClient.get(name, tempFilePath);
  console.log(`Start uploading ${name}`);
  await destOssClient.put(name, tempFilePath);
  fs.rmSync("/tmp", { recursive: true, force: true });
  callback(null, "");
};
