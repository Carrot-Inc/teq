// Program 1063 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 2663, "blank after
// template 2"): scalac 3.8.4's parser reads it as scalameta does, `class DerivationSpec { case class Foo() extends
// Bar {} { deriveEncoder[Foo] } { deriveEncoder[Foo]`, and teq's otherwise. The expectation is scalameta's tree
// under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 138
//     Defn.Object 0 137
//       Term.Name 7 12
//       Template 13 137
//         Template.Body 13 137
//           Defn.Class 15 134
//             Type.Name 21 35
//             Type.ParamClause 36 36
//             Ctor.Primary 36 36
//               Name.Anonymous 36 36
//             Template 36 134
//               Template.Body 36 134
//                 Defn.Class 40 68
//                   Mod.Case 40 44
//                   Type.Name 51 54
//                   Type.ParamClause 54 54
//                   Ctor.Primary 54 56
//                     Name.Anonymous 54 54
//                     Term.ParamClause 54 56
//                   Template 57 68
//                     Init 65 68
//                       Type.Name 65 68
//                       Name.Anonymous 68 68
//                     Template.Body 68 68
//                 Term.Block 72 100
//                   Term.ApplyType 78 96
//                     Term.Name 78 91
//                     Type.ArgClause 91 96
//                       Type.Name 92 95
//                 Term.Block 104 132
//                   Term.ApplyType 110 128
//                     Term.Name 110 123
//                     Type.ArgClause 123 128
//                       Type.Name 124 127
object W1063 {
class DerivationSpec {
  case class Foo() extends Bar

  {
    deriveEncoder[Foo]
  }

  {
    deriveEncoder[Foo]
  }
}

}
