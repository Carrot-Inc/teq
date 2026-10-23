// Program 1029 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 1524,
// "indented-enum-contructor-params"): scalac 3.8.4's parser reads it as scalameta does, `class Namespace(val uri:
// String | Null) { case object xhtml extends Namespace("http://www.w3.org/1999/xhtml") {}`, and teq's otherwise. The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 170
//     Defn.Object 0 169
//       Term.Name 7 12
//       Template 13 169
//         Template.Body 13 169
//           Defn.Enum 15 159
//             Type.Name 20 29
//             Type.ParamClause 29 29
//             Ctor.Primary 29 53
//               Name.Anonymous 29 29
//               Term.ParamClause 29 53
//                 Term.Param 30 52
//                   Mod.ValParam 30 33
//                   Term.Name 34 37
//                   Type.ApplyInfix 39 52
//                     Type.Name 39 45
//                     Type.Name 46 47
//                     Type.Name 48 52
//             Template 53 159
//               Template.Body 53 159
//                 Defn.EnumCase 57 159
//                   Term.Name 62 67
//                   Type.ParamClause 74 74
//                   Ctor.Primary 74 74
//                     Name.Anonymous 74 74
//                   Init 82 159
//                     Type.Name 82 91
//                     Name.Anonymous 118 118
//                     Term.ArgClause 127 159
//                       Lit.String 128 158
object W1029 {
enum Namespace(val uri: String | Null):
  case xhtml
      extends Namespace // Defn.EnumCase ends here
        ("http://www.w3.org/1999/xhtml") // str

}
