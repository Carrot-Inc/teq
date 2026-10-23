// Program 790 of tests/fix/scalameta, from scalameta 4.17.4's MinorDottySuite (line 440, "capital-var-pattern-
// case"): scalac 3.8.4's parser accepts it, teq's reports "expected '=>', found '@'". The expectation is scalameta's
// tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from
// the program's first line below.
//   Source 0 57
//     Defn.Object 0 56
//       Term.Name 7 11
//       Template 12 56
//         Template.Body 12 56
//           Term.Match 14 53
//             Term.Apply 14 21
//               Term.Name 14 19
//               Term.ArgClause 19 21
//             Term.CasesBlock 28 53
//               Case 32 51
//                 Pat.Bind 37 48
//                   Pat.Var 37 44
//                     Term.Name 37 44
//                   Pat.Wildcard 47 48
//                 Term.Block 52 52
object W790 {
flags() match {
  case Pattern @ _ =>
}

}
