"use client";

import { useState, useEffect } from "react";
import { useParams } from "next/navigation";

const API_BASE = "http://localhost:3000/api/v1";

interface Project {
  name: string;
  owner_id: string;
  clone_url?: string; // Assuming clone_url is available
}

interface User {
  id: string;
  name: string;
  email: string;
}

interface Permission {
  user_id: string;
  project: string;
  access: "read" | "write" | "admin";
}

export default function ProjectDetailPage() {
  const params = useParams<{ projectName: string }>();
  const projectName = params.projectName;

  const [project, setProject] = useState<Project | null>(null);
  const [users, setUsers] = useState<User[]>([]);
  const [permissions, setPermissions] = useState<Permission[]>([]);
  const [currentUser, setCurrentUser] = useState<User | null>(null);
  const [isAdmin, setIsAdmin] = useState(false);
  const [token, setToken] = useState("");

  useEffect(() => {
    const t = localStorage.getItem("token");
    if (t) {
      setToken(t);
      // Fetch current user details to check ownership/admin status
      fetch(`${API_BASE}/users/me`, {
        headers: { Authorization: `Bearer ${t}` },
      })
        .then((res) => {
          if (!res.ok) throw new Error("Authentication failed");
          return res.json();
        })
        .then((data) => {
          setCurrentUser(data);
          fetchProjectAndPermissions(t, data.id);
        })
        .catch((error) => {
          console.error("Failed to fetch current user:", error);
          fetchProjectAndPermissions(t, null); // Try fetching without user ID if auth fails
        });
    } else {
      fetchProjectAndPermissions(null, null); // Fetch project details even if not logged in
    }
  }, [projectName]); // Re-fetch when projectName changes

  const fetchProjectAndPermissions = async (
    authToken: string | null,
    currentUserId: string | null,
  ) => {
    try {
      // Fetch project details
      const projectRes = await fetch(`${API_BASE}/projects/${projectName}`, {
        headers: authToken ? { Authorization: `Bearer ${authToken}` } : {},
      });
      if (!projectRes.ok) {
        // Handle 404 or other errors if project not found
        if (projectRes.status === 404) {
          console.error("Project not found");
          // Optionally redirect or show a "not found" message
        } else {
          throw new Error(`Failed to fetch project: ${projectRes.statusText}`);
        }
        return;
      }
      const projectData: Project = await projectRes.json();
      // Use API server URL for clone command
      const apiServerUrl = "http://localhost:3000";
      projectData.clone_url =
        projectData.clone_url || `${apiServerUrl}/${projectData.name}`;
      setProject(projectData);

      // Fetch all users and permissions if logged in
      if (authToken && currentUserId) {
        const [usersRes, permsRes] = await Promise.all([
          fetch(`${API_BASE}/users`, {
            headers: { Authorization: `Bearer ${authToken}` },
          }),
          fetch(`${API_BASE}/permissions`, {
            headers: { Authorization: `Bearer ${authToken}` },
          }),
        ]);
        const [usersData, permsData]: [User[], Permission[]] =
          await Promise.all([usersRes.json(), permsRes.json()]);
        setUsers(usersData);
        setPermissions(permsData);

        // Check if current user is admin for this project
        const userPermissions = permsData.filter(
          (p: Permission) =>
            p.user_id === currentUserId && p.project === projectName,
        );
        const isAdminUser = userPermissions.some(
          (p: Permission) => p.access === "admin",
        );
        const isOwner = projectData.owner_id === currentUserId;
        setIsAdmin(isAdminUser || isOwner);
      }
    } catch (e) {
      console.error("Failed to fetch data", e);
    }
  };

  if (!project) {
    // Show a loading or not found message
    return (
      <div className="flex min-h-screen items-center justify-center bg-gray-50">
        <p className="text-lg text-gray-700">
          {projectName ? `Loading project "${projectName}"...` : "Loading..."}
        </p>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="mx-auto max-w-4xl overflow-hidden rounded-lg bg-white shadow-md">
        <div className="p-6">
          <h1 className="mb-4 text-3xl font-bold text-gray-900">
            {project.name}
          </h1>

          <div className="mb-6">
            <h2 className="mb-2 text-xl font-semibold text-gray-800">
              Project Details
            </h2>
            {project.clone_url && (
              <div className="flex items-center justify-between rounded-md bg-gray-100 p-4 shadow-inner">
                <p className="font-mono text-sm text-gray-700">
                  Clone URL:
                  <span className="ml-2 font-bold">
                    rig clone {project.clone_url}
                  </span>
                </p>
                <button
                  onClick={() =>
                    navigator.clipboard.writeText(
                      `rig clone ${project.clone_url}`,
                    )
                  }
                  className="ml-4 rounded-md bg-indigo-600 px-3 py-1 text-white hover:bg-indigo-700 focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:outline-none"
                >
                  Copy
                </button>
              </div>
            )}
          </div>

          {isAdmin && (
            <div className="mb-6">
              <h2 className="mb-2 text-xl font-semibold text-gray-800">
                Admin Actions
              </h2>
              <a
                href={`/projects/${projectName}/settings`}
                className="inline-block rounded-md bg-indigo-600 px-4 py-2 text-white hover:bg-indigo-700 focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 focus:outline-none"
              >
                Manage Permissions
              </a>
            </div>
          )}

          {/* You can add more project-specific information here */}
          <div className="mt-8">
            <h2 className="mb-2 text-xl font-semibold text-gray-800">
              Collaborators
            </h2>
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-gray-200">
                <thead className="bg-gray-50">
                  <tr>
                    <th className="px-6 py-3 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                      User
                    </th>
                    <th className="px-6 py-3 text-left text-xs font-medium tracking-wider text-gray-500 uppercase">
                      Access
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-200 bg-white">
                  {users
                    .filter((user) =>
                      permissions.some(
                        (p) =>
                          p.user_id === user.id && p.project === projectName,
                      ),
                    )
                    .map((user) => {
                      const userPerm = permissions.find(
                        (p) =>
                          p.user_id === user.id && p.project === projectName,
                      );
                      const access = userPerm ? userPerm.access : "no access";
                      return (
                        <tr key={user.id}>
                          <td className="px-6 py-4 text-sm font-medium whitespace-nowrap text-gray-900">
                            {user.name}
                          </td>
                          <td className="px-6 py-4 text-sm whitespace-nowrap text-gray-500 capitalize">
                            {access}
                          </td>
                        </tr>
                      );
                    })}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
