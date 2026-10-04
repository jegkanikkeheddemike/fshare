import { useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import { api, type ApiError } from "../api";
import { Icon } from "@fluentui/react";
import { FileIconType, getFileTypeIconProps } from "@fluentui/react-file-type-icons";
import { Disabled } from "../components/Disabled";
import { Header } from "../components/Header";

import meatball from "../assets/meatball.svg"
import { useAuth } from "react-oidc-context";

type DirEntry = {
    relative_name: string,
    is_dir: boolean,
    mime: string | null,
    loading: true | null,
}
type DirContent = {
    entries: DirEntry[]
}


type FileuploadStatus = {
    name: string,
    status: "uploading" | "finished" | "failed",
    error?: string,
}


export const BrowsePage = () => {
    const { "*": rawPath } = useParams();
    const path = rawPath || "";

    const auth = useAuth();

    const [dirContent, setDirContent] = useState<DirContent | null>(null);
    const [prevPath, setPrevPath] = useState<string | null>(null);

    const [showMenu, setShowMenu] = useState(false);
    const [fileSelectActive, setFileSelectActive] = useState(false);

    const [activeMeatball, setActiveMeatball] = useState<string | null>(null);

    const [uploadingFiles, setUploadingFiles] = useState<FileuploadStatus[] | null>(null);

    const [contentError, setContentError] = useState<ApiError | null>(null);

    const loadContent = useCallback(() => {
        api(`/dir/${path}`).then(async resp => {
            if (resp.ok) {
                setDirContent(await resp.json());
                setPrevPath(path);
                setContentError(null);
            } else {
                setContentError(await resp.json());
            }
        });
    }, [path])

    useEffect(() => {
        loadContent();
    }, [loadContent]);

    const mkdir = useCallback(async () => {
        const dc = dirContent;
        if (!dc) {
            alert("Not ready");
            return;
        }
        const name = prompt("Directory name:");
        if (!name) {
            alert("Invalid name");
            return;
        }
        if (dc.entries.find(e => e.relative_name === name)) {
            alert("File or directory already exists at this path");
            return;
        }
        setDirContent({
            entries: [...dirContent.entries, {
                is_dir: true,
                loading: true,
                mime: null,
                relative_name: name,
            }]
        })

        const resp = await api(`/mkdir/${path}${name}`, { method: "post" });
        if (!resp.ok) {

            alert(`Failed to create directory ${name}`);
            console.log("Failed to create dir:", await resp.json());
        }
        loadContent();

    }, [dirContent, path, loadContent]);


    const deleteItem = useCallback(async (relativeName: string) => {

        dirContent!.entries.find(e => e.relative_name === relativeName)!.loading = true;
        setDirContent({ entries: [...dirContent!.entries] });

        const resp = await api(`/delete/${path}${relativeName}`, { method: "post" });
        if (!resp.ok) {
            console.log("Failed to delete item with error:", await resp.json())
            alert(`Failed to delete item: ${path}${relativeName}`);
        }
        loadContent();
    }, [path, loadContent, dirContent, setDirContent]);

    const renameItem = useCallback(async (relativeName: string, newName: string) => {
        dirContent!.entries.find(e => e.relative_name === relativeName)!.loading = true;
        setDirContent({ entries: [...dirContent!.entries] });

        const resp = await api(`/rename/${path}${relativeName}`, {
            method: "post",
            body: JSON.stringify({ new_name: newName }),
        });
        if (!resp.ok) {
            const err = await resp.json();
            console.log("Failed to rename item with error:", err)
            alert(`Failed to rename item ${path}${relativeName}: ${err.detail}`);
        }
        setActiveMeatball(null);
        loadContent();
    }, [path, loadContent, dirContent, setDirContent]);

    const uploadFiles = async (
        event: React.ChangeEvent<HTMLInputElement>
    ) => {
        setFileSelectActive(false);
        if (!event.target.files) {
            return
        };
        setUploadingFiles(Array.from(event.target.files).map(f => ({ name: f.name, status: "uploading" })));

        try {
            await Promise.all(
                Array.from(event.target.files).map(file =>
                    api(`/upload/${path}${file.name}`, {
                        method: "POST",
                        body: file,
                        headers: {
                            "Content-Type":
                                file.type || "application/octet-stream",
                            "X-Filename": file.name,
                        },
                    }).then(async (resp) => {
                        if (!resp.ok) {
                            const err = await resp.json();
                            console.error(`Failed to upload file ${file.name}:`, err);
                            setUploadingFiles(prev => prev?.map(f => f.name === file.name ? { ...f, status: "failed", error: err.detail } : f) || null);
                        } else {
                            setUploadingFiles(prev => prev?.map(f => f.name === file.name ? { ...f, status: "finished" } : f) || null);
                            loadContent();
                        }

                    })
                )
            );
        } catch (e) {
            console.error("Failed to upload files:", e);
        }
    };
    return <>
        <Header />
        {!dirContent && !contentError && <div className="text-lg text-white flex-1 flex justify-center items-center">loading</div>}
        {dirContent && dirContent.entries.length === 0 && <div className="text-lg text-white flex-1 flex justify-center items-center">No items in this directory. Try upload a file or creating a directory.</div>}
        {contentError && <div className="text-white flex-1 flex flex-col justify-center items-center">
            <p className="text-xl">{contentError.status} {contentError.title}</p>
            <p className="text-base">{contentError.detail}</p>
        </div>}
        <div className="flex-1" onClick={() => setActiveMeatball(null)}>
            <div className="flex flex-wrap">
                {dirContent && dirContent.entries.map(e => <Disabled disabled={prevPath !== path} key={path + e.relative_name}>
                    <DirEntry
                        entry={e}
                        dir_path={path}
                        activeMeatball={e.relative_name === activeMeatball}
                        setActiveMeatball={() => setActiveMeatball(e.relative_name)}
                        deleteItem={deleteItem}
                        renameItem={renameItem}
                    />
                </Disabled>)}
            </div>
        </div>
        {auth.isAuthenticated &&
            <div className="flex flex-col absolute bottom-0 right-0 m-10 items-end"
                onMouseLeave={() => setShowMenu(false)}>
                {(showMenu || fileSelectActive) && <div className="flex flex-col w-40 my-4">
                    <button className="bg-gray-300 hover:bg-gray-400 cursor-pointer px-5 py-2 my-1 rounded" onClick={mkdir}>Make directory</button>
                    <input
                        id="uploads"
                        type="file"
                        className="hidden"
                        multiple
                        onChange={uploadFiles}
                        onClick={() => setFileSelectActive(true)}
                    />

                    <label
                        htmlFor="uploads"
                        className="bg-gray-300 hover:bg-gray-400 cursor-pointer px-5 py-2 my-1 rounded"
                    >Upload files</label>
                </div>}
                <div
                    className="w-20 h-20 bg-amber-300 hover:bg-amber-400 rounded-full hover:cursor-pointer flex justify-center items-center text-6xl select-none"
                    onMouseEnter={() => setShowMenu(true)}
                >+</div>
            </div>
        }
        {uploadingFiles && uploadingFiles.length > 0 && <FileUploadStatus files={uploadingFiles} clear={() => setUploadingFiles(null)} />}
    </>
}

const DirEntry = (props: { entry: DirEntry, dir_path: string, activeMeatball: boolean, setActiveMeatball: () => void, deleteItem: (relativeName: string) => void, renameItem: (relativeName: string, newName: string) => void }) => {
    const { entry, dir_path, activeMeatball, setActiveMeatball, deleteItem, renameItem } = props;

    const auth = useAuth();

    const [meatballHover, setMeatballHover] = useState(false);

    const [thumbnail, setThumbnail] = useState<string | null>(null);

    useEffect(() => {
        if (!entry.is_dir) {
            api(`/thumbnail/${dir_path}${entry.relative_name}`).then(async (resp) => {
                if (resp.ok) {
                    const blob = await resp.blob();
                    setThumbnail(URL.createObjectURL(blob));
                }
            });
        }
    }, [entry.is_dir, entry.relative_name, dir_path])


    const content = <div className={`w-64 h-24 bg-white ${!meatballHover ? "hover:bg-gray-100" : ""} rounded-xl m-4 flex flex-row justify-between`} >
        {thumbnail && <div className="overflow-hidden flex justify-center items-center p-1 max-w-36"><img src={thumbnail} className="rounded-xl" /></div>}
        {!thumbnail && !entry.is_dir && entry.mime && <Icon  {...getFileTypeIconProps({ extension: entry.mime?.split("/")[1], size: 96 })} />}
        {!thumbnail && !entry.is_dir && !entry.mime && <Icon {...getFileTypeIconProps({ type: FileIconType.genericFile, size: 96 })} />}
        {entry.is_dir && <Icon {...getFileTypeIconProps({ type: FileIconType.folder, size: 96 })} />}
        <div className="flex h-full flex-col items-end relative">
            {<div className={`overflow-hidden hover:bg-gray-200 rounded-2xl px-1 m-1 ${!auth.isAuthenticated ? "invisible" : ""}`}
                onMouseEnter={() => setMeatballHover(true)}
                onMouseLeave={() => setMeatballHover(false)}
                onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    setActiveMeatball();
                }}
            >
                <img src={meatball} width={30} style={{ marginTop: -4, marginBottom: -4 }} />
            </div>}

            {activeMeatball && <div className="w-36 absolute -right-36 top-0 z-50 rounded-2xl">
                <button className="h-12 w-full bg-gray-300 hover:bg-gray-400" onClick={(e) => {
                    e.stopPropagation();
                    e.preventDefault();
                    deleteItem(entry.relative_name);
                }}>Delete</button>
                <button className="h-12 w-full bg-gray-300 hover:bg-gray-400" onClick={(e) => {
                    e.stopPropagation();
                    e.preventDefault();
                    const newName = prompt("Enter new name:", entry.relative_name);
                    if (newName) {
                        renameItem(entry.relative_name, newName);
                    }
                }}>Rename</button>
                {/* <button className="h-12 w-full bg-gray-300 hover:bg-gray-400">Some other stuff</button>
                <button className="h-12 w-full bg-gray-300 hover:bg-gray-400">I dunno</button> */}
            </div>}
            <p className="mx-3 mb-3 text-wrap wrap-break-word max-w-36 line-clamp-2 text-ellipsis">{entry.relative_name}</p>
        </div>
    </div>

    if (entry.loading) {
        return <Disabled disabled>
            {content}
        </Disabled>
    }

    if (entry.is_dir) {
        return <Link to={entry.relative_name + "/"}>{content}</Link>
    } else {
        return <a href={`/api/file/${dir_path + entry.relative_name}`}>{content}</a>;
    }
}

const FileUploadStatus = (props: { files: FileuploadStatus[], clear: () => void }) => {
    const { files, clear } = props;

    const [dotCounter, setDotCounter] = useState(3);

    useEffect(() => {
        const interval = setInterval(() => {
            setDotCounter((prev) => (prev + 1) % 4);
        }, 500);

        return () => clearInterval(interval);
    }, []);

    const dots = (
        <>
            <span className={dotCounter >= 1 ? "visible" : "invisible"}>.</span>
            <span className={dotCounter >= 2 ? "visible" : "invisible"}>.</span>
            <span className={dotCounter >= 3 ? "visible" : "invisible"}>.</span>
        </>
    );


    return <div className="absolute bottom-0 left-0 m-10">
        <div className="bg-gray-300 rounded-xl p-4">
            <div className="flex flex-row justify-between items-center mb-2">
                <h2 className="text-lg font-bold mb-2">Uploading files</h2>
                {files.every(f => f.status !== "uploading") && <button className="px-2 py-1 bg-gray-400 hover:bg-gray-500 rounded" onClick={clear}>X</button>}
            </div>
            <ul>
                {files.map((file, index) => (
                    <li key={index} className="mb-1">
                        {file.name} - {file.status === "finished" ? "Finished" : file.status === "failed" ? `Failed: ${file.error}` : <>Uploading{dots}</>}
                    </li>
                ))}
            </ul>
        </div>
    </div>
}