"""The Java reader's broken inputs, the same for progress.py and judge.py: from 300 of the .java files of dotty's tests at
the tag ($SCALA3_TAG, else scala3-3.8.4 beside $SCALA3), taken in sorted order at an even stride, with a fixed seed:
each file cut after every k-th token (k the file's token count over 8), and each with one delimiter (a parenthesis,
bracket, brace, quote, or a comment's `/*`, `*/` or `//`) removed or doubled at 8 places drawn by the seed. Beside
them the sources of tests/java-reader/broken, each as it is, and one written here: a 5 MiB comment before a class.
Files are read and written byte for byte."""
import glob, os, random, re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
BROKEN = os.path.join(HERE, 'broken')
TOKEN = re.compile(r'"(?:\\.|[^"\\\n])*"|\'(?:\\.|[^\'\\\n])*\'|/\*.*?\*/|//[^\n]*|[A-Za-z_$][\w$]*|\d[\w.]*|\S', re.S)
DELIMS = ['(', ')', '[', ']', '{', '}', '"', "'", '/*', '*/', '//']
MAIN = 'object Main { def main(args: Array[String]): Unit = () }\n'


def tag():
    return os.environ.get('SCALA3_TAG') or os.path.join(os.path.dirname(os.environ.get('SCALA3', os.path.join(ROOT, '..', 'scala3'))), 'scala3-3.8.4')


def read(f):
    return open(f, encoding='utf-8', errors='surrogateescape', newline='').read()


def write(path, text):
    open(path, 'w', encoding='utf-8', errors='surrogateescape', newline='').write(text)


def utf8(text):
    try:
        text.encode('utf-8')
        return True
    except UnicodeEncodeError:
        return False


def generated(at):
    """(the files taken, the inputs: (key, file name, text) each, the count of broken sources among them)."""
    files = sorted(glob.glob(os.path.join(at, 'tests', '**', '*.java'), recursive=True))
    n = min(300, len(files))
    files = [files[i * len(files) // n] for i in range(n)]
    rng = random.Random(20261119)
    inputs = []
    for f in files:
        text = read(f)
        name = os.path.basename(f)
        rel = os.path.relpath(f, at).replace('/', '_')
        toks = list(TOKEN.finditer(text))
        if toks:
            k = max(1, len(toks) // 8)
            for i in range(k - 1, len(toks), k):
                inputs.append((f'cut:{rel}:{i + 1}', name, text[:toks[i].end()]))
        spots = [(m.start(), d) for d in DELIMS for m in re.finditer(re.escape(d), text)]
        for start, d in rng.sample(spots, min(8, len(spots))):
            if rng.random() < 0.5:
                inputs.append((f'drop:{rel}:{start}:{d}', name, text[:start] + text[start + len(d):]))
            else:
                inputs.append((f'double:{rel}:{start}:{d}', name, text[:start] + d + text[start:]))
    for f in sorted(glob.glob(os.path.join(BROKEN, '**', '*.java'), recursive=True)):
        inputs.append((f'broken:{os.path.relpath(f, BROKEN)}', os.path.basename(f), read(f)))
    inputs.append(('broken:large/J.java', 'J.java', '/*' + 'x' * (5 << 20) + '*/public class J {}'))
    return files, inputs, sum(1 for k, _, _ in inputs if k.startswith('broken:'))


def workdir(out, key):
    return os.path.join(out, 'w', re.sub(r'[^\w.-]', '_', key))
