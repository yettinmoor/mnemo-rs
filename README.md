# mnemo

mnemo is a terminal-based flashcard [spaced repetition system](https://en.wikipedia.org/wiki/Spaced_repetition). It is designed as a lightweight, extremely simple alternative to programs like [Anki](https://apps.ankiweb.net/).

Here is a sample of a deck file:

```
$ cat tests/test.mnemo
0 | Capital       | Country    | First letter | Founded
1 | Stockholm     | Sweden     | S            | 1252
2 | Oslo          | Norway     | O            |
3 | Washington DC | USA        | W            | 1791
4 | Antananarivo  | Madagascar | A            |
5 | Mogadishu     | Somalia    | M            |
```

Deck syntax is very simple: fields are separated by `|`. The first field is a numerical unique ID. The second field is the answer to the flashcard. The remaining fields are the cues from which the user must attempt to recall the answer. If the first row has ID 0, its fields are interpreted as field headers.

Because mnemo uses human-readable file formats, it is easy to extend with scripts. For example, using a [Jisho web scraper](https://github.com/yettinmoor/jisho-cli), it is relatively painless to turn this:

```
$ cat sentences.txt   # words to scrape marked with `「」`
「煙」は出てる。
明日の「予定」も分からない。
お前の「狙い」は俺たちなんだろう。
俺を「捕まえて」みろよ。
彼らの家の「食堂」はとても「広々」としている。
```

into this:

```
$ ./sentences sentences.txt
煙[けむり]: smoke, fumes | 「煙」は出てる。
予定[よてい]: plans, arrangement, schedule, program, programme, expectation, estimate | 明日の「予定」も分からない。
狙い[ねらい]: aim | お前の「狙い」は俺たちなんだろう。
捕まえる[つかまえる]: to catch, to capture, to arrest, to seize, to restrain | 俺を「捕まえて」みろよ。
食堂[しょくどう]: dining room, dining hall, cafeteria, canteen, messroom | 彼らの家の「食堂」はとても広々としている。
広々[ひろびろ]: extensive, spacious | 彼らの家の食堂はとても「広々」としている。
```

and append with `-a`:

```
$ ./sentences sentences.txt | mnemo japanese.mnemo -a -
appended 6 new cards to /home/nico/docs/mnemo/japanese.mnemo.
saved backup to /tmp/mnemo.

$ tail -n 6 japanese.mnemo   # let's verify it worked...
10 | 煙[けむり]: smoke, fumes | 「煙」は出てる。
11 | 予定[よてい]: plans, arrangement | 明日の「予定」も分からない。
12 | 狙い[ねらい]: aim | お前の「狙い」は俺たちなんだろう。
13 | 捕まえる[つかまえる]: to capture | 俺を「捕まえて」みろよ。
14 | 食堂[しょくどう]: dining room | 彼らの家の「食堂」はとても広々としている。
15 | 広々[ひろびろ]: spacious | 彼らの家の食堂はとても「広々」としている。
```

## Tips

Use a tool like [vim-tabular](https://github.com/godlygeek/tabular) to automatically align by `|`:

```{vimscript}
au BufWritePre *.mnemo :Tabularize /|/
```

```
$ tail -n 6 japanese.mnemo   # it's nicely aligned in the editor, but Japanese text seems to break Markdown...
10 | 煙[けむり]: smoke, fumes                | 「煙」は出てる。
11 | 予定[よてい]: plans, arrangement        | 明日の「予定」も分からない。
12 | 狙い[ねらい]: aim                       | お前の「狙い」は俺たちなんだろう。
13 | 捕まえる[つかまえる]: to capture        | 俺を「捕まえて」みろよ。
14 | 食堂[しょくどう]: dining room           | 彼らの家の「食堂」はとても広々としている。
15 | 広々[ひろびろ]: spacious                | 彼らの家の食堂はとても「広々」としている。
```
