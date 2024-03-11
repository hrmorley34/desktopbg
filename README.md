# desktopbg

A tool for randomising your desktop background/wallpaper.

## General setup

Create a directory of background images, such as
```
/path/to/bgdir/
|-- Common/
|   |-- image1.jpg
|   '-- image2.png
'-- Rare/
    |-- subfolder/
    |   '-- image3.jpeg
    '-- image4.jpg
```
and then create `bgdir/desktopbg.toml`:
```toml
[backgrounds]
# Common has a 9/10 chance of being chosen; Rare has a 1/10 chance
Common = 9
Rare = 1
```
or in shorthand:
```toml
backgrounds = { Common = 9, Rare = 1 }
```

## General usage

Run
```sh
cargo run -- /path/to/bgdir/
```
to randomise the background from those in `bgdir`.

## Windows automation

In Task Scheduler, you can create a task to run the program on login/unlock.

#### General

- Name: (choose something sensible)
- Security options

    - When running the task, use the following user account: (Your account)
    - [x] Run only when user is logged on

#### Triggers

- At log on

    - Specific user: (Your account)

- On workstation lock

    - Specific user: (Your account)

  This makes it change the wallpaper when you lock your computer; when you unlock, you'll see the updated wallpaper.

#### Actions

- Start a program

    - Program/script: `C:\path\to\desktopbg.exe`

      I use `C:\path\to\Git\desktopbg\target\release\desktopbg.exe`.
    
    - Add arguments: `C:\path\to\bgdir`
