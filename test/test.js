const crypto = require("crypto");
const os = require("os");
const path = require("path");
const axios = require("axios");
const dayjs = require("dayjs");
const lodash = require("lodash");

const numbers = [2, 4, 6, 8, 10];
const randomPick = numbers[Math.floor(Math.random() * numbers.length)];
const sha256Prefix = crypto
  .createHash("sha256")
  .update("piebash")
  .digest("hex")
  .slice(0, 16);

const payload = {
  today: dayjs().format("YYYY-MM-DD"),
  mean: numbers.reduce((a, b) => a + b, 0) / numbers.length,
  random_pick: randomPick,
  doubled_numbers: lodash.map(numbers, (value) => value * 2),
  cwd: process.cwd(),
  dirname: path.dirname(process.argv[1]),
  hostname: os.hostname(),
  platform: process.platform,
  sha256_prefix: sha256Prefix,
  axios_version: axios.VERSION,
  dayjs_version: require("dayjs/package.json").version,
  lodash_version: require("lodash/package.json").version,
};

console.log("Node.js runtime + dependency test:");
console.log(JSON.stringify(payload, null, 2));
