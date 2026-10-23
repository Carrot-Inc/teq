// Program 1002 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 837, "if-else-same-
// indent"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found 'then'". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 65
//     Defn.Object 0 64
//       Term.Name 7 12
//       Template 13 64
//         Template.Body 13 64
//           Term.If 15 61
//             Term.Block 21 33
//               Term.Name 21 25
//               Term.Name 28 33
//             Term.Block 44 51
//               Term.Name 44 46
//               Term.Name 49 51
//             Term.Name 59 61
object W1002 {
if 
  cond
  cond2
  then 
  fx
  gx
  else gx

}
