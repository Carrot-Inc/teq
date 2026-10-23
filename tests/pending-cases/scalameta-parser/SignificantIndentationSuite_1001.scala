// Program 1001 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 811, "old-try-
// catch-same-indent"): scalac 3.8.4's parser accepts it, teq's reports "expected end of statement, found 'catch'".
// The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its
// start and end in UTF-16 units from the program's first line below.
//   Source 0 89
//     Defn.Object 0 88
//       Term.Name 7 12
//       Template 13 88
//         Template.Body 13 88
//           Term.Try 15 85
//             Term.Block 22 29
//               Term.Name 22 24
//               Term.Name 27 29
//             Term.CasesBlock 42 65
//               Case 42 52
//                 Pat.Var 47 49
//                   Term.Name 47 49
//                 Term.Block 55 55
//               Case 55 65
//                 Pat.Var 60 62
//                   Term.Name 60 62
//                 Term.Block 65 65
//             Term.Block 78 85
//               Term.Name 78 80
//               Term.Name 83 85
object W1001 {
try 
  fx
  gx 
  catch 
  case aa =>
  case bb =>
  finally
  cc
  dd

}
