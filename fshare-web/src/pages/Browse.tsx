import { useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import { api } from "../api";
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

export const BrowsePage = () => {
    const { "*": rawPath } = useParams();
    const path = rawPath || "";

    const auth = useAuth();

    const [dirContent, setDirContent] = useState<DirContent | null>(null);
    const [prevPath, setPrevPath] = useState<string | null>(null);

    const [showMenu, setShowMenu] = useState(false);

    const [activeMeatball, setActiveMeatball] = useState<string | null>(null);

    const loadContent = useCallback(() => {
        api(`/dir/${path}`).then(async resp => {
            setDirContent(await resp.json());
            setPrevPath(path);
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
    return <>
        <Header />
        {!dirContent && "loading"}
        <div className="flex flex-wrap" onClick={() => setActiveMeatball(null)}>
            {dirContent && dirContent.entries.map(e => <Disabled disabled={prevPath !== path} key={path + e.relative_name}>
                <DirEntry
                    entry={e}
                    dir_path={path}
                    activeMeatball={e.relative_name === activeMeatball}
                    setActiveMeatball={() => setActiveMeatball(e.relative_name)}
                    deleteItem={deleteItem}
                />
            </Disabled>)}
        </div>
        {auth.isAuthenticated &&
            <div className="flex flex-col absolute bottom-0 right-0 m-10 items-end"
                onMouseLeave={() => setShowMenu(false)}>
                {showMenu && <div className="flex flex-col w-40 my-4">
                    <button className="bg-gray-300 hover:bg-gray-400 cursor-pointer px-5 py-2 my-1 rounded" onClick={mkdir}>Make directory</button>
                    <button className="bg-gray-300 hover:bg-gray-400 cursor-pointer px-5 py-2 my-1 rounded">Upload Files</button>
                </div>}
                <div
                    className="w-20 h-20 bg-amber-300 hover:bg-amber-400 rounded-full hover:cursor-pointer flex justify-center items-center text-6xl select-none"
                    onMouseEnter={() => setShowMenu(true)}
                >+</div>
            </div>
        }
    </>
}

const DirEntry = (props: { entry: DirEntry, dir_path: string, activeMeatball: boolean, setActiveMeatball: () => void, deleteItem: (relativeName: string) => void }) => {
    const { entry, dir_path, activeMeatball, setActiveMeatball, deleteItem } = props;

    const [meatballHover, setMeatballHover] = useState(false);

    const content = <div className={`w-64 h-24 bg-white ${!meatballHover ? "hover:bg-gray-100" : ""} rounded-xl m-4 flex flex-row justify-between`} >
        {!entry.is_dir && entry.mime && <Icon  {...getFileTypeIconProps({ extension: entry.mime?.split("/")[1], size: 96 })} />}
        {!entry.is_dir && !entry.mime && <Icon {...getFileTypeIconProps({ type: FileIconType.genericFile, size: 96 })} />}
        {entry.is_dir && <Icon {...getFileTypeIconProps({ type: FileIconType.folder, size: 96 })} />}
        <div className="flex h-full flex-col items-end relative">
            <div className="overflow-hidden hover:bg-gray-200 rounded-2xl px-1 m-1"
                onMouseEnter={() => setMeatballHover(true)}
                onMouseLeave={() => setMeatballHover(false)}
                onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    setActiveMeatball();
                }}
            >
                <img src={meatball} width={30} style={{ marginTop: -4, marginBottom: -4 }} />
            </div>
            {activeMeatball && <div className="w-36 absolute -right-36 top-0 z-50 rounded-2xl">
                <button className="h-12 w-full bg-gray-300 hover:bg-gray-400" onClick={(e) => {
                    e.stopPropagation();
                    e.preventDefault();
                    deleteItem(entry.relative_name);
                }}>Delete</button>
                <button className="h-12 w-full bg-gray-300 hover:bg-gray-400">Rename</button>
                <button className="h-12 w-full bg-gray-300 hover:bg-gray-400">Some other stuff</button>
                <button className="h-12 w-full bg-gray-300 hover:bg-gray-400">I dunno</button>
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