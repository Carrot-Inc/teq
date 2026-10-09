// A top-level definition cut at the end of the file inside a default value: the missing
// expression is the one error, and the definition, incomplete, is not reported as lacking a body.
// expect: expected an expression, found end of file
// expect: 1 error found
def broken(x: Int = (
