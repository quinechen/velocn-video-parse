"use strict";

const OSS = require("ali-oss");
const FC20230330 = require("@alicloud/fc20230330");
const OpenApi = require("@alicloud/openapi-client");
const Util = require("@alicloud/tea-util");
const Stream = require("@alicloud/darabonba-stream");

let ossClient;
let fcClient;

const sourceRegion = process.env.SOURCE_REGION;
const sourceBucket = process.env.SOURCE_BUCKET;
const workerFunctionName = process.env.WORKER_FUNCTION_NAME;

exports.initialize = async (context, callback) => {
  ossClient = new OSS({
    region: `oss-${sourceRegion}`,
    bucket: sourceBucket,
    accessKeyId: context.credentials.accessKeyId,
    accessKeySecret: context.credentials.accessKeySecret,
    stsToken: context.credentials.securityToken,
    internal: sourceRegion === context.region,
  });

  fcClient = new FC20230330.default(
    new OpenApi.Config({
      accessKeyId: context.credentials.accessKeyId,
      accessKeySecret: context.credentials.accessKeySecret,
      securityToken: context.credentials.securityToken,
      endpoint: `${context.accountId}.${context.region}-internal.fc.aliyuncs.com`,
      readTimeout: 100000,
      connectTimeout: 100000,
    })
  );

  callback(null, "");
};

exports.handler = async (event, context, callback) => {
  let continuationToken = "";
  let stop = false;
  const failedTasks = []

  while (!stop) {
    const { nextContinuationToken, objects } = await ossClient.listV2({
      "max-keys": 100,
      "continuation-token": continuationToken,
    });
    if (nextContinuationToken) {
      continuationToken = nextContinuationToken;
    } else {
      stop = true;
    }
    
    await Promise.all(
      objects.map((obj) => createObjectSyncTask(obj.name, obj.etag, failedTasks))
    );
  }

  callback(null, JSON.stringify({
    failedTasks
  }));
};

const createObjectSyncTask = async (name, etag, failedTasks) => {
  try {
    await fcClient.invokeFunctionWithOptions(
      workerFunctionName,
      new FC20230330.InvokeFunctionRequest({
        body: Stream.default.readFromString(
          JSON.stringify({
            sourceRegion,
            sourceBucket,
            name,
            etag,
          })
        ),
      }),
      new FC20230330.InvokeFunctionHeaders({
        xFcInvocationType: "Async",
      }),
      new Util.RuntimeOptions({})
    );
    console.log(`Create sync object task success for ${name}`);
  } catch (error) {
    console.error(error)
    console.log(
      `Create sync object task failed for ${name} due to "${error.data?.Message}"`
    );
    failedTasks.push(name)
  }
};
