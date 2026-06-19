import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";

type Library = {
  id: string; // UUID, pathがかぶることはないからいらないかも
  name: string;
  path: string;
};

type Content = {
  id: string; // UUID, カバー画像は末尾に拡張子をつける
  file_name: string;
  added: string;
};

export const Home = () => {
  const [librarys, setLibrarys] = useState<Library[]>([]);
  const [contents, setContents] = useState<Content[]>([]);
  const [selectedLibrarysPath, setSelectedLibrarysPath] = useState("");

  const absolutePath = useCallback(
    (fileName: string) => {
      return `${selectedLibrarysPath}/${fileName}`;
    },
    [selectedLibrarysPath],
  );

  const absoluteThumbnailPath = useCallback(
    (contentId: string) => {
      return convertFileSrc(
        `${selectedLibrarysPath}/.taureria/thumbnails/${contentId}.jpg`,
      );
    },
    [selectedLibrarysPath],
  );

  const openSubWindow = useCallback(async (path: string) => {
    await invoke("open_viewer_window", { path });
  }, []);

  const createNewLibrary = useCallback(async () => {
    const libName = "New Library";
    const result = await open({
      multiple: false,
      directory: true,
    });

    if (result === null) {
      return;
    }

    // バックエンドにライブラリの作成依頼を送信
    const res = await invoke("create", { path: result, name: libName });
    console.log("Create library result:", res);

    // ライブラリリストを更新
    await getLibraryList();
  }, []);

  // ライブラリ一覧の取得
  const getLibraryList = useCallback(async () => {
    const libraryList = await invoke<Library[]>("list");
    setLibrarys(libraryList);
  }, []);

  /**
   * ライブラリのコンテンツ一覧を取得する
   */
  const onClickLibrary = useCallback(async (path: string) => {
    console.info(`>>> ${path}`);
    setSelectedLibrarysPath(path);
    const res = await invoke<Content[]>("contents_list", {
      libraryPath: path,
    });
    setContents(res);
  }, []);

  const onClickContent = useCallback(
    async (path: string) => {
      console.info(`path: ${absolutePath(path)}`);
      await openSubWindow(absolutePath(path));
    },
    [openSubWindow, absolutePath],
  );

  const onClickDelete = useCallback(
    async (path: string) => {
      // バックエンドに削除依頼
      await invoke("delete", { libraryPath: path });

      // ライブラリリストを更新
      await getLibraryList();
    },
    [getLibraryList],
  );

  useEffect(() => {
    void (async () => {
      await getLibraryList();
    })();
  }, []);

  return (
    <div>
      <button onClick={createNewLibrary}>Create New Library</button>
      <ul>
        {librarys.map((lib) => (
          <li onClick={() => onClickLibrary(lib.path)}>
            <span>{lib.id}</span>
            <span>{lib.name}</span>
            <span>{lib.path}</span>
            <button onClick={() => onClickDelete(lib.path)}>Delete</button>
          </li>
        ))}
      </ul>
      <br />
      <div>libs</div>
      <br />
      <div>
        <ul>
          {contents.map((content) => (
            <div>
              <li onClick={() => onClickContent(content.file_name)}>
                <img src={absoluteThumbnailPath(content.id)} />
                <span>{content.file_name}</span>
                <br />
                <span>{content.added}</span>
                <br />
              </li>
            </div>
          ))}
        </ul>
      </div>
    </div>
  );
};
