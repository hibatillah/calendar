import type { CodegenConfig } from "@graphql-codegen/cli";
const config: CodegenConfig = {
  schema: "contracts/graphql/schema.graphql",
  documents: "packages/api/src/operations.graphql",
  generates: {
    "packages/api/src/generated.ts": {
      plugins: ["typescript-operations", "typed-document-node"],
    },
  },
};
export default config;
