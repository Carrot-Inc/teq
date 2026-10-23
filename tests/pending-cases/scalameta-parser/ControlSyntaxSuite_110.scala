// Program 110 of tests/fix/scalameta, from scalameta 4.17.4's ControlSyntaxSuite (line 1929, "unsure-correct"):
// scalac 3.8.4's parser accepts it, teq's reports "expected 'case', found 'catch'". The expectation is scalameta's
// tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from
// the program's first line below.
//   Source 0 76
//     Defn.Object 0 75
//       Term.Name 7 11
//       Template 12 75
//         Template.Body 12 75
//           Term.Try 14 72
//             Term.Match 18 47
//               Term.Name 18 22
//               Term.CasesBlock 31 47
//                 Case 31 47
//                   Term.Name 36 37
//                   Term.Name 41 47
//             Term.CasesBlock 56 72
//               Case 56 72
//                 Pat.Var 61 63
//                   Term.Name 61 63
//                 Term.Name 67 72
object W110 {
try func match
  case A => Accept
  catch case ex => Error

}
