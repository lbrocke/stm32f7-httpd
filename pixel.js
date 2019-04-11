'use strict';

const codes = process.argv[2].split(",").map(Number);
process.stdout.write(Buffer.from(codes));
