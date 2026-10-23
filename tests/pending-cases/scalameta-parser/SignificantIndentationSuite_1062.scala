// Program 1062 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 2623, "blank after
// template 1"): scalac 3.8.4's parser reads it as scalameta does, `class DerivationSpec { case class Foo() {} {
// deriveEncoder[Foo] } { deriveEncoder[Foo]`, and teq's otherwise. The expectation is scalameta's tree under
// dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 126
//     Defn.Object 0 125
//       Term.Name 7 12
//       Template 13 125
//         Template.Body 13 125
//           Defn.Class 15 122
//             Type.Name 21 35
//             Type.ParamClause 36 36
//             Ctor.Primary 36 36
//               Name.Anonymous 36 36
//             Template 36 122
//               Template.Body 36 122
//                 Defn.Class 40 56
//                   Mod.Case 40 44
//                   Type.Name 51 54
//                   Type.ParamClause 54 54
//                   Ctor.Primary 54 56
//                     Name.Anonymous 54 54
//                     Term.ParamClause 54 56
//                   Template 56 56
//                     Template.Body 56 56
//                 Term.Block 60 88
//                   Term.ApplyType 66 84
//                     Term.Name 66 79
//                     Type.ArgClause 79 84
//                       Type.Name 80 83
//                 Term.Block 92 120
//                   Term.ApplyType 98 116
//                     Term.Name 98 111
//                     Type.ArgClause 111 116
//                       Type.Name 112 115
object W1062 {
class DerivationSpec {
  case class Foo()

  {
    deriveEncoder[Foo]
  }

  {
    deriveEncoder[Foo]
  }
}

}
