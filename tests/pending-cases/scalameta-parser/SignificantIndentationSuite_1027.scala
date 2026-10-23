// Program 1027 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 1475,
// "indented-contructor-params"): scalac 3.8.4's parser reads it as scalameta does, `module object ExampleThing
// extends CompositeThing("One", "Two", "Three", "Four") {}`, and teq's otherwise. The expectation is scalameta's
// tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from
// the program's first line below.
//   Source 0 172
//     Defn.Object 0 171
//       Term.Name 7 12
//       Template 13 171
//         Template.Body 13 171
//           Defn.Object 15 168
//             Term.Name 22 34
//             Template 39 168
//               Init 47 168
//                 Type.Name 47 61
//                 Name.Anonymous 72 72
//                 Term.ArgClause 72 168
//                   Lit.String 90 95
//                   Lit.String 113 118
//                   Lit.String 136 143
//                   Lit.String 161 167
//               Template.Body 168 168
object W1027 {
object ExampleThing
    extends CompositeThing
          (
                "One",
                "Two",
                "Three",
                "Four")

}
