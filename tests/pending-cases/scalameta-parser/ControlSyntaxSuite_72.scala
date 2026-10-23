// Program 72 of tests/fix/scalameta, from scalameta 4.17.4's ControlSyntaxSuite (line 1201, "multiline-for"): scalac
// 3.8.4's parser accepts it, teq's reports "`yield` or `do` expected". The expectation is scalameta's tree under
// dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from the
// program's first line below.
//   Source 0 68
//     Defn.Object 0 67
//       Term.Name 7 10
//       Template 11 67
//         Template.Body 11 67
//           Term.ForYield 13 64
//             Term.EnumeratorsBlock 17 56
//               Enumerator.Generator 17 29
//                 Pat.Tuple 17 22
//                   Pat.Var 18 19
//                     Term.Name 18 19
//                   Pat.Var 20 21
//                     Term.Name 20 21
//                 Term.Name 26 29
//               Enumerator.Guard 32 40
//                 Term.ApplyInfix 35 40
//                   Term.Name 35 36
//                   Term.Name 37 38
//                   Type.ArgClause 39 39
//                   Term.ArgClause 39 40
//                     Lit.Int 39 40
//               Enumerator.Generator 43 56
//                 Pat.Var 43 44
//                   Term.Name 43 44
//                 Term.Name 48 56
//             Term.Name 63 64
object W72 {
for (a,b) <- gen
  if a < 5
  c <- otherGen
yield c

}
