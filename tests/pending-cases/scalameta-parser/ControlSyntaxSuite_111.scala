// Program 111 of tests/fix/scalameta, from scalameta 4.17.4's ControlSyntaxSuite (line 1944, "case-match-ignore-
// indent"): scalac 3.8.4's parser accepts it, teq's reports "expected 'case', found identifier". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 57
//     Defn.Object 0 56
//       Term.Name 7 11
//       Template 12 56
//         Template.Body 12 56
//           Term.Match 14 53
//             Term.Name 14 15
//             Term.CasesBlock 22 53
//               Case 26 51
//                 Pat.Var 31 32
//                   Term.Name 31 32
//                 Term.Block 41 51
//                   Term.Apply 41 44
//                     Term.Name 41 42
//                     Term.ArgClause 42 44
//                   Term.Apply 48 51
//                     Term.Name 48 49
//                     Term.ArgClause 49 51
object W111 {
x match {
  case x =>
     a()
   b()
}

}
