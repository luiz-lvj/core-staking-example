const path = require("path");
const k = require("@metaplex-foundation/kinobi");

// Paths.
const clientDir = path.join(__dirname, "", "clients");
console.log(clientDir);
const idlDir = path.join(__dirname, "target/idl");

// Instanciate Kinobi.
const kinobi = k.createFromIdls([path.join(idlDir, "core_staking_example.json")]);

// Update programs.
// kinobi.update(
//   k.updateProgramsVisitor({
//     coreStakingExample: { name: "coreStakingExample" },
//   })
// );

// Render JavaScript.
const jsDir = path.join(clientDir, "js", "src", "generated");
kinobi.accept(k.renderJavaScriptVisitor(jsDir));