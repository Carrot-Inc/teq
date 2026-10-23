// Program 935 of tests/fix/scalameta, from scalameta 4.17.4's PatSuite (line 68, "pat: F[t & u | v]()"): scalac
// 3.8.4's parser accepts it, teq's reports "expected '=>', found '['". The expectation is scalameta's tree under
// dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 72
//     Defn.Object 0 71
//       Term.Name 7 11
//       Template 12 71
//         Template.Body 12 71
//           Defn.Def 16 69
//             Term.Name 20 21
//             Member.ParamClauseGroup 21 29
//               Type.ParamClause 21 21
//               Term.ParamClause 21 29
//                 Term.Param 22 28
//                   Term.Name 22 23
//                   Type.Name 25 28
//             Term.Match 32 69
//               Term.Name 32 33
//               Term.CasesBlock 40 69
//                 Case 42 67
//                   Pat.Extract 47 61
//                     Term.ApplyType 47 59
//                       Term.Name 47 48
//                       Type.ArgClause 48 59
//                         Type.ApplyInfix 49 58
//                           Type.ApplyInfix 49 54
//                             Type.Name 49 50
//                             Type.Name 51 52
//                             Type.Name 53 54
//                           Type.Name 55 56
//                           Type.Name 57 58
//                     Pat.ArgClause 59 61
//                   Lit.Unit 65 67
object W935 {
  def w(x: Any) = x match {
case F[t & u | v]() => ()
}
}
