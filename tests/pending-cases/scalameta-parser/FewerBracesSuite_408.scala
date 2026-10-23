// Program 408 of tests/fix/scalameta, from scalameta 4.17.4's FewerBracesSuite (line 2297, "#4152 not fewer braces:
// extension with type params"): scalac 3.8.4's parser accepts it, teq's reports "expected ',' or ']', found ':'".
// The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its
// start and end in UTF-16 units from the program's first line below.
//   Source 0 74
//     Defn.Object 0 73
//       Term.Name 7 11
//       Template 12 73
//         Template.Body 12 73
//           Defn.ExtensionGroup 14 70
//             Member.ParamClauseGroup 24 56
//               Type.ParamClause 24 40
//                 Type.Param 25 39
//                   Type.Name 25 26
//                   Type.ParamClause 26 26
//                   Type.Bounds 26 39
//                     Type.Name 36 39
//               Term.ParamClause 40 56
//                 Term.Param 41 55
//                   Term.Name 41 42
//                   Type.Name 52 55
//             Defn.Def 59 70
//               Term.Name 63 66
//               Term.Name 69 70
object W408 {
extension [B:
        Seq](b:
        Int)
  def foo = b

}
