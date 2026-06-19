import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { useLocation } from "react-router";

export const Viewer = () => {
  const location = useLocation();
  const urlParams = new URLSearchParams(location.search);

  // const [images, setImages] = useState([] as string[]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [currentData, setCurrentData] = useState<string | null>(null);
  // ページカウントがnullの場合、バックエンドの準備ができていないとみなす
  const [pageCount, setPageCount] = useState<number | null>(null);

  /**
   * バックエンドに対してコマンドを送って1ページを取得する
   */
  const loadImage = useCallback(async (index: number) => {
    // if (index < 0 || index >= images.length) return;
    const res = (await invoke("get_image", { index })) as {
      data: string;
      mime: string;
    };
    console.info(JSON.stringify(res));
    // The response has {data, mime}
    const dataUrl = `data:${res.mime};base64,${res.data}`;
    setCurrentData(dataUrl);
  }, []);

  /**
   * ページ送りのハンドラ
   */
  const keyHandler = useCallback(
    (e: KeyboardEvent) => {
      if (!pageCount) return;

      if (e.key === "ArrowRight" && currentIndex < pageCount - 1) {
        setCurrentIndex((i) => i + 1);
      } else if (e.key === "ArrowLeft" && currentIndex > 0) {
        setCurrentIndex((i) => i - 1);
      }
    },
    [currentIndex, pageCount],
  );

  /**
   * バックエンドでデータの準備が出来たら呼ばれる関数
   */
  const onDataReady = useCallback(() => {
    const pageCount = urlParams.get("p") as unknown as number;
    // ページ数をセット
    setPageCount(pageCount); // ページがセットされると画像がリクエストされる
    // 現在のページを0にセット
    setCurrentIndex(0);
  }, []);

  // バックエンドでデータの準備が完了したら送られてくるデータをリッスン
  useEffect(() => {
    onDataReady();
  }, []);

  useEffect(() => {
    window.addEventListener("keydown", keyHandler);
    return () => window.removeEventListener("keydown", keyHandler);
  }, [keyHandler]);

  /**
   * 画像データそのものの取得
   */
  useEffect(() => {
    if (pageCount) loadImage(currentIndex);
  }, [currentIndex, pageCount, loadImage]);

  return (
    <div>
      {currentData ? (
        <img
          src={currentData}
          alt={`Image ${currentIndex + 1}`}
          style={{ maxWidth: "100%" }}
        />
      ) : (
        <div>Loading...</div>
      )}
    </div>
  );
};
