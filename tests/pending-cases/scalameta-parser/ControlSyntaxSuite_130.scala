// Program 130 of tests/fix/scalameta, from scalameta 4.17.4's ControlSyntaxSuite (line 2562, "sub-case: match-as-
// operator guard with => stays a guard"): scalac 3.8.4's parser accepts it, teq's reports "expected '=>', found
// 'match'". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a
// line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 76
//     Defn.Object 0 75
//       Term.Name 7 11
//       Template 12 75
//         Template.Body 12 75
//           Term.Match 14 73
//             Term.Name 14 15
//             Term.CasesBlock 22 73
//               Case 24 71
//                 Pat.Var 29 32
//                   Term.Name 29 32
//                 Term.Match 36 66
//                   Term.Name 36 39
//                   Term.CasesBlock 46 66
//                     Case 48 64
//                       Pat.Var 53 56
//                         Term.Name 53 56
//                       Lit.Boolean 60 64
//                 Lit.Int 70 71
object W130 {
x match { case foo if bar match { case baz => true } => 1 }
}
