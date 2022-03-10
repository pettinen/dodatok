"use strict";
exports.__esModule = true;
var ajv_1 = require("ajv");
var ajv = new ajv_1["default"]();
var schema = {
    type: "object",
    properties: {
        x: { type: "string" }
    }
};
var validate = ajv.compile(schema);
