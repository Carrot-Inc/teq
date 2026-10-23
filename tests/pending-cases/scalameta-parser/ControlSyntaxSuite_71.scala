// Program 71 of tests/fix/scalameta, from scalameta 4.17.4's ControlSyntaxSuite (line 1183, "for-new"): scalac
// 3.8.4's parser accepts it, teq's reports "`yield` or `do` expected". The expectation is scalameta's tree under
// dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 72
//     Defn.Object 0 71
//       Term.Name 7 10
//       Template 11 71
//         Template.Body 11 71
//           Term.ForYield 13 68
//             Term.EnumeratorsBlock 17 57
//               Enumerator.Generator 17 25
//                 Pat.Var 17 18
//                   Term.Name 17 18
//                 Term.Name 22 25
//               Enumerator.Val 30 35
//                 Pat.Var 30 31
//                   Term.Name 30 31
//                 Lit.Int 34 35
//               Enumerator.Guard 40 57
//                 Term.ApplyInfix 43 57
//                   Term.Name 44 48
//                   Term.Name 50 52
//                   Type.ArgClause 53 53
//                   Term.ArgClause 53 57
//                     Term.Name 53 57
//             Term.Name 64 68
object W71 {
for i <- gen
    x = 3
    if (cnd1) && cnd2
yield work

}
