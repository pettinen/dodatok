import Ajv, {JSONSchemaType} from "ajv";

const ajv = new Ajv();

interface A {
  x?: string;
}

const schema: JSONSchemaType<A> = {
  type: "object",
  properties: {
    x: { type: "string" },
  },
};

const validate = ajv.compile(schema);
