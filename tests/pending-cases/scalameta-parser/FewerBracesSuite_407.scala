// Program 407 of tests/fix/scalameta, from scalameta 4.17.4's FewerBracesSuite (line 2281, "#4152 not fewer braces:
// extension 1"): scalac 3.8.4's parser accepts it, teq's reports "expected extension methods". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 58
//     Defn.Object 0 57
//       Term.Name 7 11
//       Template 12 57
//         Template.Body 12 57
//           Defn.ExtensionGroup 14 54
//             Member.ParamClauseGroup 24 40
//               Type.ParamClause 24 24
//               Term.ParamClause 24 40
//                 Term.Param 25 39
//                   Term.Name 25 26
//                   Type.Name 36 39
//             Defn.Def 43 54
//               Term.Name 47 50
//               Term.Name 53 54
object W407 {
extension (b:
        Int)
  def foo = b

}
