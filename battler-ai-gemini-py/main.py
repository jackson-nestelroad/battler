import argparse
import json
import mimetypes
import os
import pathlib
import re
from collections.abc import Callable
from typing import Optional

from dotenv import load_dotenv
from google import genai
from google.genai import types


# Program options.
class Options:
    def __init__(self, args: dict):
        self.working_dir = pathlib.Path(os.getcwd())
        self.binary_dir = pathlib.Path(__file__).parent

        self.player = validate_player(args.player)
        self.input = validate_input(args.input)

        self.use_cache = args.use_cache

        self.data = validate_data(args.data) if args.data else None
        self.use_data_files = args.use_data_files

    working_dir: pathlib.Path
    binary_dir: pathlib.Path

    player: str
    input: str

    use_cache: bool

    data: Optional[pathlib.Path]
    use_data_files: bool


def validate_player(player: str) -> str:
    if not player:
        raise NameError("Player ID is not defined")
    if not re.findall(r"^[\w-]+$", player):
        raise ValueError(f"Player is not an alphanumeric ID")
    return player


def validate_input(input: str) -> dict:
    if not input:
        raise NameError("Input is not defined")
    try:
        input = json.loads(input)
    except Exception as e:
        raise ValueError("Battle input is not valid JSON") from e

    if not isinstance(input, dict):
        raise ValueError("Invalid battle input JSON") from e

    if not "player_data" in input:
        raise ValueError("Battle input is missing player data")
    if not "battle_state" in input:
        raise ValueError("Battle input is missing battle state")
    if not "request_data" in input:
        raise ValueError("Battle input is missing request data")
    if not "failed_actions" in input:
        raise ValueError("Battle input is missing failed actions")

    return json.dumps(input)


def validate_data(data: pathlib.Path) -> pathlib.Path:
    if not data:
        raise NameError("Data directory is not defined")
    if not data.exists():
        raise ValueError("Data directory does not exist")
    if not data.is_dir():
        raise ValueError("Data directory is not a directory")
    return data


# Replaces input of the form "${{ KEY }}" with the given value.
def prompt_input(prompt: str, key: str, value: str) -> str:
    key = re.escape(key)
    return re.sub(rf"\$\{{\{{\s+{key}\s+\}}\}}", lambda _: value, prompt)


def read_file(dir: pathlib.Path, file: str) -> str:
    path = dir / file
    if not path.exists():
        raise FileNotFoundError(f"File {file} does not exist")
    with path.open() as f:
        return f.read()


# Ensures the given file is uploaded.
def ensure_file(
    client: genai.Client, display_name: str, path: pathlib.Path
) -> types.File:
    for file in client.files.list():
        if file.display_name == display_name:
            print(f"File {path} is already uploaded")
            return file

    print(f"Uploading {path}")
    mime_type, _ = mimetypes.guess_type(path)
    file = client.files.upload(
        file=path,
        config=types.UploadFileConfig(display_name=display_name, mime_type=mime_type),
    )
    return file


# Ensures all files in the given directory are uploaded.
def ensure_all_files_in_directory(
    client: genai.Client,
    prefix: str,
    dir: pathlib.Path,
    filter: Callable[[pathlib.Path], bool],
) -> list[types.File]:
    print(f"Ensuring all files in {dir} are uploaded")
    files = []
    for path in dir.rglob("*"):
        if path.is_file() and filter(path):
            name = re.sub(r"\W+", "_", str(path.relative_to(dir)))
            files.append(ensure_file(client, f"{prefix}_{name}", path))
    return files


# Ensures the given cache exists.
def ensure_cache(
    client: genai.Client,
    model: str,
    display_name: str,
    context: str,
    files: Optional[list[types.File]],
) -> types.CachedContent:
    for cache in client.caches.list():
        if cache.display_name == display_name:
            print(f"Context cache {display_name} already exists")
            return cache
    print(f"Creating context cache {display_name}")
    cache = client.caches.create(
        model=model,
        config=types.CreateCachedContentConfig(
            display_name=display_name,
            system_instruction=context,
            contents=files,
            ttl="300s",
        ),
    )
    return cache


def write_debug_file(options: Options, name: str, content: str):
    try:
        debug_dir = options.binary_dir / ".debug"
        os.makedirs(debug_dir, exist_ok=True)
        with (debug_dir / name).open("w") as file:
            file.write(content)
    except Exception:
        pass


def gemini_battler(options: Options, context: str, prompt: str):
    client = genai.Client(api_key=os.getenv("GEMINI_API_KEY"))

    model = client.models.get(model="models/gemini-2.5-flash")

    print(
        f"Starting Gemini x battler on {model.name} (input token limit = {model.input_token_limit})"
    )

    write_debug_file(options, "prompt", prompt)

    files = None
    if options.use_data_files:
        if not options.use_cache:
            raise ValueError("Caching must be enabled if using data files")
        if not options.data:
            raise ValueError("Data directory must be defined if using data files")
        files = ensure_all_files_in_directory(
            client=client,
            prefix="data",
            dir=options.data,
            filter=lambda path: path.suffix == ".json",
        )

    config = types.GenerateContentConfig(system_instruction=context)

    if options.use_cache:
        cache = ensure_cache(
            client=client,
            model=model.name,
            display_name="battler",
            context=context,
            files=files,
        )
        config = types.GenerateContentConfig(cached_content=cache.name)

    print("Generating content...")

    response = client.models.generate_content(
        model=model.name,
        contents=prompt,
        config=config,
    )

    write_debug_file(options, "response", json.dumps(response.to_json_dict()))

    return response.text


def str_to_bool(val: str):
    if val.lower() in {"false", "f", "0", "no", "n"}:
        return False
    elif val.lower() in {"true", "t", "1", "yes", "y"}:
        return True
    raise ValueError(f"{val} is not a valid boolean value")


def run(options: Options):
    load_dotenv(options.binary_dir / ".env")

    context = read_file(options.binary_dir, "context.md")
    prompt = read_file(options.binary_dir, "prompt.md")

    prompt = prompt_input(prompt, "PLAYER", options.player)
    prompt = prompt_input(prompt, "INPUT", options.input)

    output = gemini_battler(options=options, context=context, prompt=prompt)
    print("Output")
    print(f"{output}")


def main():
    parser = argparse.ArgumentParser(description="battler AI via Gemini")

    parser.add_argument("--player", type=str, help="Player ID")
    parser.add_argument("--input", type=str, help="Battle input, in JSON form")

    parser.add_argument(
        "--use_cache",
        type=str_to_bool,
        help="Whether or not to use context caching",
        default=True,
    )

    parser.add_argument(
        "--data",
        type=pathlib.Path,
        help="Battle data directory; only required if sending data files",
    )
    parser.add_argument(
        "--use_data_files",
        type=bool,
        help="Whether or not to include all data files (not recommended, as input token count gets too high)",
        default=False,
    )

    args = parser.parse_args()

    run(Options(args))


if __name__ == "__main__":
    main()
