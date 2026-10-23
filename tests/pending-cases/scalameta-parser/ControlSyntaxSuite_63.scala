// Program 63 of tests/fix/scalameta, from scalameta 4.17.4's ControlSyntaxSuite (line 1052,
// "new-for-indented-without-do2"): scalac 3.8.4's parser reads it as scalameta does, `for a <- x do { fx fy`, and
// teq's otherwise. The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a
// node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 41
//     Defn.Object 0 40
//       Term.Name 7 10
//       Template 11 40
//         Template.Body 11 40
//           Term.For 13 37
//             Term.EnumeratorsBlock 17 27
//               Enumerator.Generator 19 25
//                 Pat.Var 19 20
//                   Term.Name 19 20
//                 Term.Name 24 25
//             Term.Block 30 37
//               Term.Name 30 32
//               Term.Name 35 37
object W63 {
for { a <- x }
  fx
  fy

}
