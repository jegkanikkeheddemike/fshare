import { useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router";
import { api } from "../api";
import { Icon } from "@fluentui/react";
import { FileIconType, getFileTypeIconProps } from "@fluentui/react-file-type-icons";
import { Disabled } from "../components/Disabled";
import { Header } from "../components/Header";
import keycloak from "../keycloak";

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

    const [dirContent, setDirContent] = useState<DirContent | null>(null);
    const [prevPath, setPrevPath] = useState<string | null>(null);

    const [showMenu, setShowMenu] = useState(false);

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

    return <>
        <Header />
        {!dirContent && "loading"}
        <div className="flex flex-wrap">
            {dirContent && dirContent.entries.map(e => <Disabled disabled={prevPath !== path} key={path + e.relative_name}>
                <DirEntry entry={e} dir_path={path} />
            </Disabled>)}
        </div>
        {keycloak.authenticated &&
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

const DirEntry = (props: { entry: DirEntry, dir_path: string }) => {
    const { entry, dir_path } = props;

    const content = <div className="w-64 h-24 bg-white hover:bg-gray-100 rounded-xl m-4 flex flex-row justify-between" >
        {!entry.is_dir && entry.mime && <Icon  {...getFileTypeIconProps({ extension: entry.mime?.split("/")[1], size: 96 })} />}
        {!entry.is_dir && !entry.mime && <Icon {...getFileTypeIconProps({ type: FileIconType.genericFile, size: 96 })} />}
        {entry.is_dir && <Icon {...getFileTypeIconProps({ type: FileIconType.folder, size: 96 })} />}
        <div className="flex h-full flex-col items-end">

            <div className="mr-2 mt-2 bg-pink-500 w-10 h-5">
            </div>
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